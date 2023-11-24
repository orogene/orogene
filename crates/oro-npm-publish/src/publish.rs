use crate::error::OroNpmPublishError;
use base64::{engine::general_purpose, Engine as _};
use futures::io::{AsyncRead, AsyncReadExt};
use open::that as open;
use oro_client::login::DoneURLResponse;
use oro_client::{publish::PublishResponse, OTPResponse, OroClient};
use oro_common::{Access, Attachments, Dist, Manifest, Packument, PublishConfig, VersionMetadata};
use sha1::{Digest, Sha1};
use ssri::{Algorithm, IntegrityOpts};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

pub struct PublishOptions {
    pub access: Access,
    pub default_tag: String,
    pub algorithms: Vec<Algorithm>,
    pub client: OroClient,
}

pub async fn publish(
    manifest: &Manifest,
    mut tarball: impl AsyncRead + Unpin,
    options: PublishOptions,
) -> Result<(), OroNpmPublishError> {
    if let Some(true) = manifest.private {
        return Err(OroNpmPublishError::PrivatePackageError);
    }
    let manifest_name = manifest
        .name
        .clone()
        .ok_or(OroNpmPublishError::RequiredFieldIsMissing(
            "name".to_owned(),
        ))?;
    let manifest_version =
        manifest
            .version
            .clone()
            .ok_or(OroNpmPublishError::RequiredFieldIsMissing(
                "version".to_owned(),
            ))?;
    let manifest_tag = manifest.tag.clone().unwrap_or(options.default_tag);
    let tarball_name = format!("{manifest_name}-{manifest_version}.tgz");
    let tarball_uri = format!("{manifest_name}/-/{tarball_name}");
    let integrity_opts = IntegrityOpts::new();
    let mut integrity_opts = integrity_opts.algorithm(Algorithm::Sha1);
    let mut tarball_data = Vec::new();
    tarball.read_to_end(&mut tarball_data).await?;
    let tarball_data = Arc::new(tarball_data);
    let cloned_tarball = tarball_data.clone();

    for algorithm in options.algorithms {
        integrity_opts = integrity_opts.algorithm(algorithm);
    }

    integrity_opts.input(&*tarball_data);
    let mut metadata = VersionMetadata::default();
    let mut dist = Dist::default();
    let mut tarball_url = options.client.registry.join(&tarball_uri)?;
    let integrity = integrity_opts.result();
    let mut hashes = integrity.hashes.into_iter();
    let _ = tarball_url.set_scheme("http");
    dist.shasum = Some(
        async_std::task::spawn_blocking(move || {
            let mut hasher = Sha1::new();
            hasher.update(&*cloned_tarball);
            hex::encode(hasher.finalize())
        })
        .await,
    );
    dist.integrity = Some(
        hashes
            .find(|hash| hash.algorithm == Algorithm::Sha512)
            .unwrap()
            .to_string(),
    );
    dist.tarball = Some(tarball_url.clone());
    metadata.id = Some(format!("{manifest_name}@{manifest_version}"));
    metadata.dist = dist;
    metadata.manifest = manifest.clone();

    let mut packument = Packument {
        id: Some(manifest_name.clone()),
        name: Some(manifest_name.clone()),
        description: manifest.description.clone(),
        access: Some(options.access),
        attachments: HashMap::new(),
        versions: HashMap::new(),
        time: HashMap::new(),
        tags: HashMap::new(),
        rest: HashMap::new(),
    };
    packument
        .versions
        .insert(manifest_version.clone(), metadata);
    packument
        .tags
        .insert(manifest_tag, manifest_version.clone());
    packument.attachments.insert(
        tarball_name,
        Attachments {
            content_type: "application/octet-stream".to_owned(),
            data: general_purpose::STANDARD.encode(&*tarball_data),
            length: tarball_data.len(),
        },
    );

    match options
        .client
        .publish(&manifest_name, &packument, None)
        .await?
    {
        PublishResponse::OTPRequired(OTPResponse::WebOTP { auth_url, done_url }) => {
            open(auth_url).map_err(OroNpmPublishError::OpenURLError)?;

            loop {
                match options.client.fetch_done_url(&done_url).await? {
                    DoneURLResponse::Token(token) => {
                        match options
                            .client
                            .publish(&manifest_name, &packument, Some(&token))
                            .await?
                        {
                            PublishResponse::Success => break Ok(()),
                            PublishResponse::OTPRequired(_) => {
                                break Err(OroNpmPublishError::ReceivedUnexpectedResponse)
                            }
                        }
                    }
                    DoneURLResponse::Duration(duration) => {
                        async_std::task::sleep(duration).await;
                    }
                }
            }
        }
        PublishResponse::OTPRequired(_) | PublishResponse::Success => Ok(()),
    }
}

impl Default for PublishOptions {
    fn default() -> Self {
        Self {
            access: Access::Public,
            algorithms: vec![Algorithm::Sha512],
            default_tag: "latest".to_owned(),
            client: OroClient::new(Url::parse("https://registry.npmjs.org").unwrap()),
        }
    }
}

impl PublishOptions {
    pub fn merge(self, config: PublishConfig) -> Self {
        Self {
            access: config.access.unwrap_or(self.access),
            default_tag: config.default_tag.unwrap_or(self.default_tag),
            ..self
        }
    }

    pub fn maybe_merge(self, config: Option<PublishConfig>) -> Self {
        if let Some(config) = config {
            return self.merge(config);
        }
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_compression::futures::bufread::GzipEncoder;
    use futures::AsyncSeekExt;
    use miette::{IntoDiagnostic, Result};
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[async_std::test]
    async fn publish_test() -> Result<()> {
        let mock_server = MockServer::start().await;
        let mock_server_uri: Url = mock_server.uri().parse().into_diagnostic()?;
        let client = OroClient::new(mock_server_uri.clone());
        let package_name = utf8_percent_encode("oro-test-package", NON_ALPHANUMERIC).to_string();
        let _guard = Mock::given(method("PUT"))
            .and(path(&package_name))
            .and(body_json(&serde_json::json!({
                "_id": "oro-test-package",
                "name": "oro-test-package",
                "versions": {
                    "0.0.1": {
                        "dist": {
                            "shasum": "d26b15dcffb3f8de4b386808d1435a75b6fc307f",
                            "tarball": mock_server_uri.join("oro-test-package/-/oro-test-package-0.0.1.tgz").unwrap(),
                            "integrity": "sha512-yPkJSAw+lmaRlH4QiZHfAs/c36XtIM85UHOZeeSAmarFnt/gJT4cBayRkW1LbknjLjOWIuPkSfGlY0sBXUDGWA=="
                        },
                        "_id": "oro-test-package@0.0.1",
                        "name": "oro-test-package",
                        "version": "0.0.1"
                    }
                },
                "time": {},
                "dist-tags": { "latest": "0.0.1" },
                "access": "public",
                "_attachments": {
                    "oro-test-package-0.0.1.tgz": {
                        "content_type": "application/octet-stream",
                        "data": "H4sIAAAAAAAA/+3AAQEAAACCIP+vbkhQwKsBLq+17wAEAAA=",
                        "length": 35
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;
        let manifest = Manifest {
            name: Some("oro-test-package".to_owned()),
            version: Some("0.0.1".parse()?),
            ..Default::default()
        };
        let tarball_data = {
            let mut cursor = async_std::io::Cursor::new(Vec::new());
            {
                let mut builder = async_tar_wasm::Builder::new(&mut cursor);
                builder.finish().await.into_diagnostic()?;
            }
            let _ = cursor
                .seek(async_std::io::SeekFrom::Start(0))
                .await
                .into_diagnostic()?;
            Box::new(GzipEncoder::new(cursor))
        };

        assert!(matches!(
            publish(
                &manifest,
                tarball_data,
                PublishOptions {
                    client,
                    ..Default::default()
                }
            )
            .await,
            Ok(())
        ));

        Ok(())
    }
}
