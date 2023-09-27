use crate::notify::Notify;
use crate::{OroClient, OroClientError};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::{HeaderMap, WWW_AUTHENTICATE};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum DoneURLResponse {
    Token(String),
    Duration(Duration),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthType {
    Web,
    Legacy,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LoginCouchResponse {
    WebOTP { auth_url: String, done_url: String },
    ClassicOTP,
    Token(String),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Token {
    pub token: String,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginWeb {
    pub login_url: String,
    pub done_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct LoginOptions {
    pub scope: Option<String>,
    pub client: Option<OroClient>,
}

#[derive(Deserialize, Serialize)]
struct LoginCouch {
    _id: String,
    name: String,
    password: String,
    r#type: String,
    roles: Vec<String>,
    date: String,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct WebOTPResponse {
    auth_url: Option<String>,
    done_url: Option<String>,
}

impl OroClient {
    fn build_header(auth_type: AuthType, options: &LoginOptions) -> HeaderMap {
        let mut headers = HashMap::new();

        if let Some(scope) = options.scope.clone() {
            headers.insert("npm-scope".to_owned(), scope);
        }

        headers.insert(
            "npm-auth-type".to_owned(),
            match auth_type {
                AuthType::Web => "web".to_owned(),
                AuthType::Legacy => "legacy".to_owned(),
            },
        );
        headers.insert("npm-command".to_owned(), "login".to_owned());
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());

        (&headers)
            .try_into()
            .expect("This type conversion should work")
    }

    pub async fn login_web(&self, options: &LoginOptions) -> Result<LoginWeb, OroClientError> {
        let headers = Self::build_header(AuthType::Web, options);
        let url = self.registry.join("-/v1/login")?;
        let text = self
            .client
            .post(url.clone())
            .headers(headers)
            .header("X-Oro-Registry", self.registry.to_string())
            .send()
            .await?
            .notify()
            .error_for_status()?
            .text()
            .await?;

        serde_json::from_str::<LoginWeb>(&text)
            .map_err(|e| OroClientError::from_json_err(e, url.to_string(), text))
    }

    pub async fn login_couch(
        &self,
        username: &str,
        password: &str,
        otp: Option<&str>,
        options: &LoginOptions,
    ) -> Result<LoginCouchResponse, OroClientError> {
        let mut headers = Self::build_header(AuthType::Legacy, options);
        let username_ = utf8_percent_encode(username, NON_ALPHANUMERIC).to_string();
        let url = self
            .registry
            .join(&format!("-/user/org.couchdb.user:{username_}"))?;

        if let Some(otp) = otp {
            headers.insert(
                "npm-otp",
                otp.try_into().expect("This type conversion should work"),
            );
        }

        let response = self
            .client
            .put(url.clone())
            .header("X-Oro-Registry", self.registry.to_string())
            .headers(headers)
            .body(
                serde_json::to_string(&LoginCouch {
                    _id: format!("org.couchdb.user:{username}"),
                    name: username.to_owned(),
                    password: password.to_owned(),
                    r#type: "user".to_owned(),
                    roles: vec![],
                    date: chrono::Local::now().to_rfc3339(),
                })
                .expect("This type conversion should work"),
            )
            .send()
            .await?
            .notify();

        match response.status() {
            StatusCode::BAD_REQUEST => Err(OroClientError::NoSuchUserError(username.to_owned())),
            StatusCode::UNAUTHORIZED => {
                let www_authenticate = response
                    .headers()
                    .get(WWW_AUTHENTICATE)
                    .map_or(String::default(), |header| {
                        header.to_str().unwrap().to_lowercase()
                    });

                let text = response.text().await?;
                let json = serde_json::from_str::<WebOTPResponse>(&text).unwrap_or_default();

                if www_authenticate.contains("otp") || text.to_lowercase().contains("one-time pass")
                {
                    if otp.is_none() {
                        if let (Some(auth_url), Some(done_url)) = (json.auth_url, json.done_url) {
                            Ok(LoginCouchResponse::WebOTP { auth_url, done_url })
                        } else {
                            Ok(LoginCouchResponse::ClassicOTP)
                        }
                    } else {
                        Err(OroClientError::OTPRequiredError)
                    }
                } else {
                    Err(if www_authenticate.contains("basic") {
                        OroClientError::IncorrectPasswordError
                    } else if www_authenticate.contains("bearer") {
                        OroClientError::InvalidTokenError
                    } else {
                        OroClientError::ResponseError(Some(text).into())
                    })
                }
            }
            _ if response.status() >= StatusCode::BAD_REQUEST => Err(
                OroClientError::ResponseError(Some(response.text().await?).into()),
            ),
            _ => {
                let text = response.text().await?;
                Ok(LoginCouchResponse::Token(
                    serde_json::from_str::<Token>(&text)
                        .map_err(|e| OroClientError::from_json_err(e, url.to_string(), text))?
                        .token,
                ))
            }
        }
    }

    pub async fn fetch_done_url(
        &self,
        done_url: impl AsRef<str>,
    ) -> Result<DoneURLResponse, OroClientError> {
        let headers = Self::build_header(AuthType::Web, &LoginOptions::default());

        let response = self
            .client_uncached
            .get(done_url.as_ref())
            .header("X-Oro-Registry", self.registry.to_string())
            .headers(headers)
            .send()
            .await?
            .notify();

        match response.status() {
            StatusCode::OK => {
                let text = response.text().await?;
                Ok(DoneURLResponse::Token(
                    serde_json::from_str::<Token>(&text)
                        .map_err(|e| {
                            OroClientError::from_json_err(e, done_url.as_ref().to_string(), text)
                        })?
                        .token,
                ))
            }
            StatusCode::ACCEPTED => {
                if let Some(retry_after) = response.headers().get("retry-after") {
                    let retry_after = retry_after.to_str()
                        .expect("The \"retry-after\" header that's included in the response should be string.")
                        .parse::<u64>()
                        .expect("The \"retry-after\" header that's included in the response should be able to parse to number.");
                    Ok(DoneURLResponse::Duration(Duration::from_secs(retry_after)))
                } else {
                    Err(OroClientError::ResponseError(
                        Some(response.text().await?).into(),
                    ))
                }
            }
            _ => Err(OroClientError::ResponseError(
                Some(response.text().await?).into(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{body_json_schema, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[async_std::test]
    async fn login_web() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);

        let body = LoginWeb {
            login_url: "https://example.com/login?next=/login/cli/foo".to_owned(),
            done_url: "https://registry.example.org/-/v1/done?sessionId=foo".to_owned(),
        };

        {
            let _guard = Mock::given(method("POST"))
                .and(path("-/v1/login"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(client.login_web(&LoginOptions::default()).await?, body);
        }

        Ok(())
    }

    #[async_std::test]
    async fn login_couch() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);

        {
            let body = Token {
                token: "XXXXXX".to_owned(),
            };

            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .and(header("npm-scope", "@mycompany"))
                .and(body_json_schema::<LoginCouch>)
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client
                    .login_couch(
                        "test",
                        "password",
                        None,
                        &LoginOptions {
                            scope: Some("@mycompany".to_owned()),
                            client: None,
                        }
                    )
                    .await?,
                LoginCouchResponse::Token(body.token),
                "Works with credentials"
            );
        }

        {
            let body = WebOTPResponse {
                auth_url: Some("https://example.com/login?next=/login/cli/foo".to_owned()),
                done_url: Some("https://registry.example.org/-/v1/done?sessionId=foo".to_owned()),
            };

            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .and(body_json_schema::<LoginCouch>)
                .respond_with(
                    ResponseTemplate::new(401)
                        .append_header("www-authenticate", "OTP")
                        .set_body_json(&body),
                )
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client
                    .login_couch("test", "password", None, &LoginOptions::default())
                    .await?,
                LoginCouchResponse::WebOTP {
                    auth_url: body.auth_url.unwrap(),
                    done_url: body.done_url.unwrap()
                }
            )
        }

        {
            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .and(body_json_schema::<LoginCouch>)
                .respond_with(ResponseTemplate::new(401).set_body_string("One-time pass"))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client
                    .login_couch("test", "password", None, &LoginOptions::default())
                    .await?,
                LoginCouchResponse::ClassicOTP
            )
        }

        {
            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(200).set_body_string(""))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client
                        .login_couch("test", "password", None, &LoginOptions::default())
                        .await,
                    Err(OroClientError::BadJson { .. })
                ),
                "If the response has no \"token\" key and the status code is 200, this will fail"
            );
        }

        {
            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(400))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client
                        .login_couch("test", "password", None, &LoginOptions::default())
                        .await,
                    Err(OroClientError::NoSuchUserError(_))
                ),
                "If the status code is 400, the client returns \"NoSuchUserError\""
            );
        }

        {
            let _guard = Mock::given(method("PUT"))
                .and(path("-/user/org.couchdb.user:test"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(503))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client
                        .login_couch("test", "password", None, &LoginOptions::default())
                        .await,
                    Err(OroClientError::ResponseError(_))
                ),
                "If the status code is 402 or higher, this will fail"
            );
        }

        Ok(())
    }

    #[async_std::test]
    async fn fetch_done_url() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);
        let done_url = client.registry.join("-/v1/done").unwrap();
        let done_url = done_url.as_str();

        {
            let body = Token {
                token: "XXXXXXX".to_owned(),
            };

            let _guard = Mock::given(method("GET"))
                .and(path("-/v1/done"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client.fetch_done_url(done_url).await?,
                DoneURLResponse::Token(body.token)
            );
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/v1/done"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(202).append_header("retry-after", "5"))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert_eq!(
                client.fetch_done_url(done_url).await?,
                DoneURLResponse::Duration(Duration::from_secs(5)),
                "Works with \"retry-after\" header"
            );
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/v1/done"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client.fetch_done_url(done_url).await,
                    Err(OroClientError::BadJson { .. })
                ),
                "If the response has no \"token\" key and the status code is 200, this will fail"
            )
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/v1/done"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(202))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client.fetch_done_url(done_url).await,
                    Err(OroClientError::ResponseError(_))
                ),
                "If the retry-after header is not set and the status code is 202, this will fail"
            );
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("-/v1/done"))
                .and(header_exists("npm-auth-type"))
                .and(header_exists("npm-command"))
                .respond_with(ResponseTemplate::new(503))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(
                matches!(
                    client.fetch_done_url(done_url).await,
                    Err(OroClientError::ResponseError(_))
                ),
                "If the status code is not 200 or 202, this will fail"
            );
        }

        Ok(())
    }
}
