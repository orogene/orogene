use std::collections::HashMap;
use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use oro_client::{self, OroClient};
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::{NassunError, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub(crate) struct NpmFetcher {
    client: OroClient,
    registries: HashMap<Option<String>, Url>,
    cache_packuments: bool,
    packuments: DashMap<String, Arc<Packument>>,
    corgi_packuments: DashMap<String, Arc<CorgiPackument>>,
}

impl NpmFetcher {
    pub(crate) fn new(
        client: OroClient,
        registries: HashMap<Option<String>, Url>,
        cache_packuments: bool,
    ) -> Self {
        Self {
            client,
            registries,
            packuments: DashMap::new(),
            corgi_packuments: DashMap::new(),
            cache_packuments,
        }
    }
}

impl NpmFetcher {
    fn pick_registry(&self, scope: &Option<String>) -> Url {
        self.registries
            .get(scope)
            .or_else(|| self.registries.get(&None))
            .cloned()
            .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
    }
}

impl NpmFetcher {
    fn _name<'a>(&'a self, spec: &'a PackageSpec) -> &'a str {
        match spec {
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => name,
            _ => unreachable!(),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PackageFetcher for NpmFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        Ok(self._name(spec).to_string())
    }

    async fn corgi_metadata(&self, pkg: &Package) -> Result<CorgiVersionMetadata> {
        let wanted = match pkg.resolved() {
            PackageResolution::Npm { ref version, .. } => version,
            _ => unreachable!(),
        };
        let packument = self.corgi_packument(pkg.from(), Path::new("")).await?;
        packument
            .versions
            .get(wanted)
            .cloned()
            .ok_or_else(|| NassunError::MissingVersion(pkg.from().clone(), wanted.clone()))
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        let wanted = match pkg.resolved() {
            PackageResolution::Npm { ref version, .. } => version,
            _ => unreachable!(),
        };
        let packument = self.packument(pkg.from(), Path::new("")).await?;
        packument
            .versions
            .get(wanted)
            .cloned()
            .ok_or_else(|| NassunError::MissingVersion(pkg.from().clone(), wanted.clone()))
    }

    async fn corgi_packument(
        &self,
        spec: &PackageSpec,
        _base_dir: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        if let PackageSpec::Npm {
            ref name,
            ref scope,
            ..
        } = spec.target()
        {
            if let Some(packument) = self.corgi_packuments.get(name) {
                if self.cache_packuments {
                    return Ok(packument.value().clone());
                }
            }
            let client = self.client.with_registry(self.pick_registry(scope));
            let packument = Arc::new(client.corgi_packument(&name).await?);
            if self.cache_packuments {
                self.corgi_packuments
                    .insert(name.clone(), packument.clone());
            }
            Ok(packument)
        } else {
            unreachable!("How did a non-Npm resolution get here?");
        }
    }

    async fn packument(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<Arc<Packument>> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        let pkg = match spec {
            PackageSpec::Alias { ref spec, .. } => spec,
            pkg @ PackageSpec::Npm { .. } => pkg,
            _ => unreachable!(),
        };
        if let PackageSpec::Npm {
            ref name,
            ref scope,
            ..
        } = pkg
        {
            if let Some(packument) = self.packuments.get(name) {
                if self.cache_packuments {
                    return Ok(packument.value().clone());
                }
            }
            let client = self.client.with_registry(self.pick_registry(scope));
            let packument = Arc::new(client.packument(&name).await?);
            if self.cache_packuments {
                self.packuments.insert(name.clone(), packument.clone());
            }
            Ok(packument)
        } else {
            unreachable!()
        }
    }

    async fn tarball(&self, pkg: &Package) -> Result<crate::TarballStream> {
        let url = match pkg.resolved() {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(self.client.stream_external(url).await?)
    }
}

#[cfg(test)]
mod test {
    use oro_package_spec::VersionSpec;
    use tempfile::tempdir;

    use super::*;

    #[async_std::test]
    async fn read_name() -> miette::Result<()> {
        let fetcher = NpmFetcher::new(oro_client::OroClient::default(), HashMap::default(), false);
        let spec = PackageSpec::Npm {
            scope: None,
            name: "npm".to_string(),
            requested: Some(VersionSpec::Range(">1.0.0".parse()?)),
        };
        let cache_path = tempdir().unwrap();
        let name = fetcher.name(&spec, cache_path.path()).await?;
        assert_eq!(name, "npm");
        Ok(())
    }

    #[async_std::test]
    async fn read_packument() -> miette::Result<()> {
        let mut mock_server = mockito::Server::new();
        let example_response = format!(
            r#"{{
            "_attachments": {{}},
            "_id": "oro-test-example",
            "_rev": "1-0da57asf9e977d423ed4a35aedc17d250",
            "author": {{
                "email": "oro-test@example.com",
                "name": "Orogene Test"
            }},
            "description": "Example Orogene package",
            "dist-tags": {{
                "latest": "1.0.0"
            }},
            "license": "ISC",
            "maintainers": [
                {{
                    "email": "oro-test@example.com",
                    "name": "oro-test"
                }}
            ],
            "name": "oro-test-example",
            "readme": "Nothing much",
            "readmeFilename": "README.md",
            "time": {{
                "1.0.0": "2023-03-13T12:33:00.044Z",
                "created": "2023-03-13T12:33:00.045Z",
                "modified": "2023-03-13T12:33:00.046Z"
            }},
            "versions": {{
                "1.0.0": {{
                    "_from": ".",
                    "_id": "oro-test-example@1.0.0",
                    "_nodeVersion": "1.5.0",
                    "_npmUser": {{
                        "email": "oro-test@example.com",
                        "name": "oro-test"
                    }},
                    "_npmVersion": "2.7.0",
                    "_shasum": "bbf102d5ae73afe2c553295e0fb02230216f65b1",
                    "author": {{
                        "email": "oro-test@example.com",
                        "name": "oro-test"
                    }},
                    "description": "The first version of the Orogene Test Example",
                    "directories": {{}},
                    "dist": {{
                        "shasum": "bbf102d5ae73afe2c553295e0fb02230216f65b1",
                        "tarball": "{}/oro-test-example/-/oro-test-example-1.0.0.tgz"
                    }},
                    "license": "ISC",
                    "main": "index.js",
                    "maintainers": [
                        {{
                            "email": "oro-test@example.com",
                            "name": "oro-test"
                        }}
                    ],
                    "name": "oro-test-example",
                    "scripts": {{
                        "test": "echo \"Error: no test specified\" && exit 1"
                    }},
                    "version": "1.0.0"
                }}
            }}
        }}"#,
            mock_server.url()
        );
        mock_server
            .mock("GET", "/oro-test-example")
            .with_body(example_response)
            .create_async()
            .await;

        let mut registries = HashMap::new();
        registries.insert(None, Url::parse(mock_server.url().as_ref()).unwrap());

        let fetcher = NpmFetcher::new(oro_client::OroClient::default(), registries, false);
        let spec = PackageSpec::Npm {
            scope: None,
            name: "oro-test-example".to_string(),
            requested: Some(VersionSpec::Range(">=1.0.0".parse()?)),
        };
        let cache_path = tempdir().unwrap();
        let packument = fetcher.packument(&spec, cache_path.path()).await?;
        assert!(packument
            .versions
            .contains_key(&"1.0.0".parse()?));
        let mut tags = HashMap::new();
        tags.insert("latest".to_string(), "1.0.0".parse()?);
        assert_eq!(packument.tags, tags);
        assert_eq!(
            packument
                .versions
                .get(&"1.0.0".parse()?)
                .unwrap()
                .dist
                .tarball,
            Some(
                Url::parse(
                    format!(
                        "{}/oro-test-example/-/oro-test-example-1.0.0.tgz",
                        mock_server.url()
                    )
                    .as_str()
                )
                .unwrap()
            )
        );
        Ok(())
    }
}
