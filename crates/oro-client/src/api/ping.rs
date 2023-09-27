use crate::{OroClient, OroClientError};

impl OroClient {
    pub async fn ping(&self) -> Result<String, OroClientError> {
        Ok(self
            .client
            .get(self.registry.join("-/ping?write=true")?)
            .header("X-Oro-Registry", self.registry.to_string())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }
}

#[cfg(test)]
mod test {
    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[async_std::test]
    async fn ping() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);

        {
            let body = json!({
                "ok": true,
                "uptime": 1234,
                "now": "2021-01-01T00:00:00.000Z"
            });

            let _guard = Mock::given(method("GET"))
                .and(path("-/ping"))
                .and(query_param("write", "true"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(client.ping().await?, body.to_string(), "Works with JSON");
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/ping"))
                .and(query_param("write", "true"))
                .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client.ping().await?,
                "ok",
                "Plain text responses work fine too."
            );
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/ping"))
                .and(query_param("write", "true"))
                .respond_with(ResponseTemplate::new(404))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(client.ping().await, Err(OroClientError::RequestError(_))),
                "404s actually fail with an error"
            );
        }

        Ok(())
    }
}
