use oro_common::{CorgiPackument, Packument};
use reqwest::{StatusCode, Url};
use reqwest_middleware::RequestBuilder;

use crate::{credentials::Credentials, OroClient, OroClientError};

pub(crate) const CORGI_HEADER: &str =
    "application/vnd.npm.install-v1+json; q=1.0,application/json; q=0.8,*/*";

impl OroClient {
    pub async fn packument(
        &self,
        package_name: impl AsRef<str>,
    ) -> Result<Packument, OroClientError> {
        let url = self.registry.join(package_name.as_ref())?;
        tracing::trace!(
            "fetching packument for {} from {}",
            package_name.as_ref(),
            url
        );
        let text = self.packument_impl(package_name, &url, false).await?;
        serde_json::from_str(&text)
            .map_err(move |e| OroClientError::from_json_err(e, url.to_string(), text))
    }

    pub async fn corgi_packument(
        &self,
        package_name: impl AsRef<str>,
    ) -> Result<CorgiPackument, OroClientError> {
        let url = self.registry.join(package_name.as_ref())?;
        let text = self.packument_impl(package_name, &url, true).await?;
        serde_json::from_str(&text)
            .map_err(move |e| OroClientError::from_json_err(e, url.to_string(), text))
    }

    async fn packument_impl(
        &self,
        package_name: impl AsRef<str>,
        url: &Url,
        use_corgi: bool,
    ) -> Result<String, OroClientError> {
        let client = self.client.get(url.clone()).header(
            "Accept",
            if use_corgi {
                CORGI_HEADER
            } else {
                "application/json"
            },
        );
        Ok(self
            .with_credentials(url, client)
            .send()
            .await?
            .error_for_status()
            .map_err(|err| {
                if err.status() == Some(StatusCode::NOT_FOUND) {
                    OroClientError::PackageNotFound(
                        (*self.registry).clone(),
                        package_name.as_ref().to_string(),
                    )
                } else {
                    OroClientError::RequestError(err)
                }
            })?
            .text()
            .await?)
    }

    fn with_credentials(&self, url: &Url, builder: RequestBuilder) -> RequestBuilder {
        let credentials = url.host_str().and_then(|h| self.credentials.get(h));
        if let Some(cred) = credentials {
            match cred {
                Credentials::Basic(data) => {
                    builder.basic_auth(&data.username, Some(&data.password))
                }
                Credentials::Token(token) => builder.bearer_auth(token),
            }
        } else {
            builder
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use std::collections::HashMap;

    use indexmap::IndexMap;
    use maplit::hashmap;
    use miette::{IntoDiagnostic, Result};
    use oro_common::{CorgiManifest, CorgiVersionMetadata, Manifest, VersionMetadata};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{header, headers, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::OroClientBuilder;

    use super::*;

    #[async_std::test]
    async fn packument_fetch() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client = OroClient::new(mock_server.uri().parse().into_diagnostic()?);

        Mock::given(method("GET"))
            .and(path("some-pkg"))
            .and(headers("accept", CORGI_HEADER.split(',').collect()))
            .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
                "versions": {
                    "1.0.0": {
                        "name": "some-pkg",
                        "version": "1.0.0",
                        "dependencies": {
                            "some-dep": "1.0.0"
                        }
                    }
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_eq!(
            client.corgi_packument("some-pkg").await?,
            CorgiPackument {
                versions: hashmap!(
                    "1.0.0".parse()? => CorgiVersionMetadata {
                        manifest: CorgiManifest {
                            name: Some("some-pkg".to_string()),
                            version: Some("1.0.0".parse()?),
                            dependencies: IndexMap::from([
                                ("some-dep".to_string(), "1.0.0".to_string())
                            ]),
                            ..Default::default()
                        },
                    ..Default::default()
                }),
                ..Default::default()
            }
        );

        Mock::given(method("GET"))
            .and(path("some-pkg"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
                "versions": {
                    "1.0.0": {
                        "name": "some-pkg",
                        "version": "1.0.0",
                        "dependencies": {
                            "some-dep": "1.0.0"
                        }
                    }
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_eq!(
            client.packument("some-pkg").await?,
            Packument {
                versions: hashmap!(
                    "1.0.0".parse()? => VersionMetadata {
                        manifest: Manifest {
                            name: Some("some-pkg".to_string()),
                            version: Some("1.0.0".parse()?),
                            dependencies: IndexMap::from([
                                ("some-dep".to_string(), "1.0.0".to_string())
                            ]),
                            ..Default::default()
                        },
                    ..Default::default()
                }),
                ..Default::default()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn fetch_with_credentials() -> Result<()> {
        let mock_server = MockServer::start().await;
        let url: Url = mock_server.uri().parse().into_diagnostic()?;
        let host = url.host_str().unwrap();
        let cred_config = vec![
            (
                host.to_string(),
                "username".to_string(),
                "testuser".to_string(),
            ),
            (
                host.to_string(),
                "password".to_string(),
                "testpassword".to_string(),
            ),
        ];
        let client = OroClient::builder()
            .registry(url)
            .credentials(cred_config)?
            .build();

        Mock::given(method("GET"))
            .and(path("some-pkg"))
            .and(header("accept", "application/json"))
            .and(header(
                "authorization",
                "Basic dGVzdHVzZXI6dGVzdHBhc3N3b3Jk",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
                "versions": {
                    "1.0.0": {
                        "name": "some-pkg",
                        "version": "1.0.0",
                        "dependencies": {
                            "some-dep": "1.0.0"
                        }
                    }
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_eq!(
            client.packument("some-pkg").await?,
            Packument {
                versions: hashmap!(
                    "1.0.0".parse()? => VersionMetadata {
                        manifest: Manifest {
                            name: Some("some-pkg".to_string()),
                            version: Some("1.0.0".parse()?),
                            dependencies: IndexMap::from([
                                ("some-dep".to_string(), "1.0.0".to_string())
                            ]),
                            ..Default::default()
                        },
                    ..Default::default()
                }),
                ..Default::default()
            }
        );

        Ok(())
    }
}
