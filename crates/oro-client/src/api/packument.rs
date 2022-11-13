use oro_common::Packument;
use reqwest::StatusCode;

use crate::{OroClient, OroClientError};

pub(crate) const CORGI_HEADER: &str =
    "application/vnd.npm.install-v1+json; q=1.0,application/json; q=0.8,*/*";

impl OroClient {
    pub async fn packument(
        &self,
        package_name: impl AsRef<str>,
        use_corgi: bool,
    ) -> Result<Packument, OroClientError> {
        let url = self.registry.join(package_name.as_ref())?;
        let text = self
            .client
            .get(url.clone())
            .header(
                "Accept",
                if use_corgi {
                    CORGI_HEADER
                } else {
                    "application/json"
                },
            )
            .send()
            .await?
            .error_for_status()
            .map_err(|err| {
                if err.status() == Some(StatusCode::NOT_FOUND) {
                    OroClientError::PackageNotFound(package_name.as_ref().to_string())
                } else {
                    OroClientError::RequestError(err)
                }
            })?
            .text()
            .await?;
        serde_json::from_str(&text)
            .map_err(move |e| OroClientError::from_json_err(e, url.to_string(), text))
    }
}

#[cfg(test)]
mod test {
    use maplit::hashmap;
    use miette::{IntoDiagnostic, Result};
    use oro_common::{Manifest, VersionMetadata};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{header, headers, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            client.packument("some-pkg", true).await?,
            Packument {
                versions: hashmap!(
                    "1.0.0".parse()? => VersionMetadata {
                        manifest: Manifest {
                            name: Some("some-pkg".to_string()),
                            version: Some("1.0.0".parse()?),
                            dependencies: hashmap!(
                                "some-dep".to_string() => "1.0.0".to_string()
                            ),
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
            client.packument("some-pkg", false).await?,
            Packument {
                versions: hashmap!(
                    "1.0.0".parse()? => VersionMetadata {
                        manifest: Manifest {
                            name: Some("some-pkg".to_string()),
                            version: Some("1.0.0".parse()?),
                            dependencies: hashmap!(
                                "some-dep".to_string() => "1.0.0".to_string()
                            ),
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
