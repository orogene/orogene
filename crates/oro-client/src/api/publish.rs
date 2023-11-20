use crate::authentication_helper::{AuthenticationHelper, OTPResponse};
use crate::traits::{Notify, Otp};
use crate::{OroClient, OroClientError};
use oro_common::Packument;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::StatusCode;

#[derive(Debug, Clone, PartialEq)]
pub enum PublishResponse {
    Success,
    OTPRequired(OTPResponse),
}

impl OroClient {
    pub async fn publish(
        &self,
        package_name: &str,
        packument: &Packument,
        otp: Option<&str>,
    ) -> Result<PublishResponse, OroClientError> {
        let package_name = utf8_percent_encode(package_name, NON_ALPHANUMERIC).to_string();

        let response = self
            .client_uncached
            .put(self.registry.join(&package_name)?)
            .header("X-Oro-Registry", self.registry.to_string())
            .header("Content-Type", "application/json")
            .header("npm-auth-type", "web")
            .body(serde_json::to_string(&packument).expect("This type conversion should work"))
            .otp(otp)
            .send()
            .await?
            .notify();

        match response.status() {
            StatusCode::OK => Ok(PublishResponse::Success),
            StatusCode::UNAUTHORIZED => Ok(PublishResponse::OTPRequired(
                AuthenticationHelper::check_response(response, otp).await?,
            )),
            _ => Err(OroClientError::from_response(response).await?),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;
    use wiremock::matchers::{body_json, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[async_std::test]
    async fn publish() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);
        let package_name = utf8_percent_encode("oro-test-package", NON_ALPHANUMERIC).to_string();

        let body = Packument {
            name: Some("oro-test-package".to_owned()),
            ..Default::default()
        };

        {
            let _guard = Mock::given(method("PUT"))
                .and(path(&package_name))
                .and(header_exists("npm-otp"))
                .and(body_json(&body))
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client
                    .publish("oro-test-package", &body, Some("otp"))
                    .await?,
                PublishResponse::Success
            );
        }

        Ok(())
    }
}
