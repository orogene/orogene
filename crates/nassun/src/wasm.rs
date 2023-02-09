//! WASM-oriented Nassun interface for more idiomatic JS usage.

use std::collections::HashMap;

use futures::StreamExt;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

use crate::error::NassunError;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
/**
 * Error type thrown by the Nassun API.
 */
export interface NassunError {
    message: string;
    code?: string;
}

/**
 * An entry extracted from a package tarball.
 */
export interface Entry {
    type: number;
    mtime: number;
    size: number;
    path: string;
    contents: ReadableStream<Uint8Array>;
}
"#;

type Result<T> = std::result::Result<T, NassunError>;

impl From<NassunError> for JsValue {
    fn from(e: NassunError) -> Self {
        let obj = js_sys::Object::new();
        let msg = format!("{e}");
        js_sys::Reflect::set(&obj, &"message".into(), &JsValue::from_str(&msg))
            .expect(&format!("failed to set error message: {e}"));
        if let Some(code) = e.code() {
            let code = format!("{code}");
            js_sys::Reflect::set(&obj, &"code".into(), &JsValue::from_str(&code))
                .expect(&format!("failed to set error code: {e:#?}"));
        }
        obj.into()
    }
}

/// Resolves a `Packument` for the given package `spec`.
///
/// This uses default `Nassun` options and does not cache the result.
/// To configure `Nassun`, and/or enable more efficient caching/reuse,
/// look at `Package#packument` instead.
#[wasm_bindgen]
pub async fn packument(spec: &str, opts: JsValue) -> Result<JsValue> {
    Nassun::new(opts)?.resolve(spec).await?.packument().await
}

/// Resolves a partial ("corgi") version of the `Packument` for the given
/// package `spec`.
///
/// This uses default `Nassun` options and does not cache the result.
/// To configure `Nassun`, and/or enable more efficient caching/reuse,
/// look at `Package#packument` instead.
#[wasm_bindgen(js_name = "corgiPackument")]
pub async fn corgi_packument(spec: &str, opts: JsValue) -> Result<JsValue> {
    Nassun::new(opts)?
        .resolve(spec)
        .await?
        .corgi_packument()
        .await
}

/// Resolves version metadata from the given package `spec`, using the default
/// resolution algorithm.
///
/// This uses default `Nassun` options and does not cache the result. To
/// configure `Nassun`, and/or enable more efficient caching/reuse, look at
/// `Package#metadata` instead.
#[wasm_bindgen]
pub async fn metadata(spec: &str, opts: JsValue) -> Result<JsValue> {
    Nassun::new(opts)?.resolve(spec).await?.metadata().await
}

/// Resolves a partial ("corgi") version of the version metadata from the
/// given package `spec`, using the default resolution algorithm.
///
/// This uses default `Nassun` settings and does not cache the result. To
/// configure `Nassun`, and/or enable more efficient caching/reuse, look at
/// `Package#metadata` instead.
#[wasm_bindgen(js_name = "corgiMetadata")]
pub async fn corgi_metadata(spec: &str, opts: JsValue) -> Result<JsValue> {
    Nassun::new(opts)?
        .resolve(spec)
        .await?
        .corgi_metadata()
        .await
}

/// Resolves a tarball from the given package `spec`, using the
/// default resolution algorithm. This tarball will have its data checked
/// if the package metadata fetched includes integrity information.
///
/// This uses default `Nassun` settings and does not cache the result.
/// To configure `Nassun`, and/or enable more efficient caching/reuse,
/// look at `Package#tarball` instead.
#[wasm_bindgen]
pub async fn tarball(
    spec: &str,
    opts: JsValue,
) -> Result<wasm_streams::readable::sys::ReadableStream> {
    Nassun::new(opts)?.resolve(spec).await?.tarball().await
}

/// Resolves to a `ReadableStream<Entry>` of entries from the given package
/// `spec`, using the default resolution algorithm. The source tarball will
/// have its data checked if the package metadata fetched includes integrity
/// information.
///
/// This uses default `Nassun` settings and does not cache the result. To
/// configure `Nassun`, and/or enable more efficient caching/reuse, look at
/// `Package#entries` instead.
#[wasm_bindgen]
pub async fn entries(
    spec: &str,
    opts: JsValue,
) -> Result<wasm_streams::readable::sys::ReadableStream> {
    Nassun::new(opts)?.resolve(spec).await?.entries().await
}

/// Options for configuration for various `Nassun` operations.
#[derive(Debug, Deserialize, Tsify)]
#[allow(non_snake_case)]
struct NassunOpts {
    /// Registry to use for unscoped packages, and as a default for scoped
    /// packages. Defaults to `https://registry.npmjs.org/`.
    #[tsify(optional)]
    pub registry: Option<String>,
    /// A map of scope prefixes to registries.
    #[tsify(optional)]
    pub scopedRegistries: Option<HashMap<String, String>>,
}

/// NPM package client used to resolve and fetch package data and metadata.
#[wasm_bindgen]
pub struct Nassun {
    #[wasm_bindgen(skip)]
    pub inner: crate::client::Nassun,
}

impl Nassun {
    fn new_inner(inner: crate::client::Nassun) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen]
impl Nassun {
    /// Create a new Nassun instance with the given options.
    #[wasm_bindgen(constructor)]
    pub fn new(opts: JsValue) -> Result<Nassun> {
        console_error_panic_hook::set_once();
        let mut opts_builder = crate::client::NassunOpts::new();
        let opts: Option<NassunOpts> = serde_wasm_bindgen::from_value(opts)?;
        if let Some(opts) = opts {
            if let Some(registry) = opts.registry {
                opts_builder = opts_builder.registry(registry.parse()?);
            }
            if let Some(scopes) = opts.scopedRegistries {
                for (scope, registry) in scopes {
                    opts_builder = opts_builder.scope_registry(scope, registry.parse()?);
                }
            }
        }
        Ok(Nassun::new_inner(opts_builder.build()))
    }

    /// Resolve a spec (e.g. `foo@^1.2.3`, `github:foo/bar`, etc), to a
    /// `Package` that can be used for further operations.
    pub async fn resolve(&self, spec: &str) -> Result<Package> {
        Ok(Package::from_core_package(self.inner.resolve(spec).await?))
    }

    /// Resolves a packument object for the given package `spec`.
    pub async fn packument(&self, spec: &str) -> Result<JsValue> {
        self.resolve(spec).await?.packument().await
    }

    /// Resolves version metadata from the given package `spec`.
    pub async fn metadata(&self, spec: &str) -> Result<JsValue> {
        self.resolve(spec).await?.metadata().await
    }

    /// Resolves a partial (corgi) version of the packument object for the
    /// given package `spec`.
    #[wasm_bindgen(js_name = "corgiPackument")]
    pub async fn corgi_packument(&self, spec: &str) -> Result<JsValue> {
        self.resolve(spec).await?.corgi_packument().await
    }

    /// Resolves a partial (corgi) version of the version metadata from the
    /// given package `spec`.
    #[wasm_bindgen(js_name = "corgiMetadata")]
    pub async fn corgi_metadata(&self, spec: &str) -> Result<JsValue> {
        self.resolve(spec).await?.corgi_metadata().await
    }

    /// Resolves a `ReadableStream<Uint8Array>` tarball from the given package
    /// `spec`. This tarball will have its data checked if the package
    /// metadata fetched includes integrity information.
    pub async fn tarball(&self, spec: &str) -> Result<wasm_streams::readable::sys::ReadableStream> {
        self.resolve(spec).await?.tarball().await
    }

    /// Resolves to a `ReadableStream<Entry>` of entries from the given package
    /// `spec`, using the default resolution algorithm. The source tarball will
    /// have its data checked if the package metadata fetched includes integrity
    /// information.
    pub async fn entries(&self, spec: &str) -> Result<wasm_streams::readable::sys::ReadableStream> {
        self.resolve(spec).await?.entries().await
    }
}

/// A resolved package. A concrete version has been determined from its
/// PackageSpec by the version resolver.
#[wasm_bindgen]
pub struct Package {
    #[wasm_bindgen(skip)]
    pub from: JsValue,
    #[wasm_bindgen(skip)]
    pub name: JsValue,
    #[wasm_bindgen(skip)]
    pub resolved: JsValue,
    package: crate::package::Package,
    serializer: serde_wasm_bindgen::Serializer,
}

impl Package {
    pub fn from_core_package(package: crate::package::Package) -> Package {
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        Package {
            from: JsValue::from_str(&package.from().to_string()),
            name: JsValue::from_str(package.name()),
            resolved: JsValue::from_str(&format!("{}", package.resolved())),
            package,
            serializer,
        }
    }
}

#[wasm_bindgen]
impl Package {
    /// Original package spec that this `Package` was resolved from.
    #[wasm_bindgen(getter)]
    pub fn from(&self) -> JsValue {
        self.from.clone()
    }

    /// Name of the package, as it should be used in the dependency graph.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> JsValue {
        self.name.clone()
    }

    /// The package resolution information that this `Package` was created from.
    #[wasm_bindgen(getter)]
    pub fn resolved(&self) -> JsValue {
        self.resolved.clone()
    }

    /// The partial (corgi) version of the packument that this `Package` was
    /// resolved from.
    #[wasm_bindgen(js_name = "corgiPackument")]
    pub async fn corgi_packument(&self) -> Result<JsValue> {
        Ok(self
            .package
            .corgi_packument()
            .await?
            .serialize(&self.serializer)?)
    }

    /// The partial (corgi) version of the version metadata, aka roughly the
    /// metadata defined in `package.json`.
    #[wasm_bindgen(js_name = "corgiMetadata")]
    pub async fn corgi_metadata(&self) -> Result<JsValue> {
        Ok(self
            .package
            .corgi_metadata()
            .await?
            .serialize(&self.serializer)?)
    }

    /// The full packument that this `Package` was resolved from.
    pub async fn packument(&self) -> Result<JsValue> {
        Ok(self
            .package
            .packument()
            .await?
            .serialize(&self.serializer)?)
    }

    /// The version metadata, aka roughly the metadata defined in
    /// `package.json`.
    pub async fn metadata(&self) -> Result<JsValue> {
        Ok(self.package.metadata().await?.serialize(&self.serializer)?)
    }

    /// A `ReadableStream<Uint8Array>` tarball for this package. This tarball
    /// will have its data checked if the package metadata fetched includes
    /// integrity information.
    pub async fn tarball(&self) -> Result<wasm_streams::readable::sys::ReadableStream> {
        Ok(ReadableStream::from_async_read(self.package.tarball().await?, 1024).into_raw())
    }

    /// A `ReadableStream<Entry>` of entries for this package. The source
    /// tarball will have its data checked if the package metadata fetched
    /// includes integrity information.
    pub async fn entries(&self) -> Result<wasm_streams::readable::sys::ReadableStream> {
        let entries = self.package.entries().await?.then(|entry| async move {
            entry
                .map_err(|e| e.into())
                .and_then(
                    |entry: crate::entries::Entry| -> std::result::Result<JsValue, JsValue> {
                        let header = entry.header();
                        let obj = js_sys::Object::new();
                        js_sys::Reflect::set(
                            &obj,
                            &"type".into(),
                            &header.entry_type().as_byte().into(),
                        )?;
                        js_sys::Reflect::set(
                            &obj,
                            &"mtime".into(),
                            &header
                                .mtime()
                                .map(|mut x| {
                                    if x > (u32::MAX as u64) {
                                        x = u32::MAX as u64;
                                    }
                                    x as u32
                                })
                                .map_err(|e| -> NassunError { e.into() })?
                                .into(),
                        )?;
                        js_sys::Reflect::set(
                            &obj,
                            &"size".into(),
                            &header
                                .entry_size()
                                .map(|mut x| {
                                    if x > (u32::MAX as u64) {
                                        x = u32::MAX as u64;
                                    }
                                    x as u32
                                })
                                .map_err(|e| -> NassunError { e.into() })?
                                .into(),
                        )?;
                        js_sys::Reflect::set(
                            &obj,
                            &"path".into(),
                            &entry.path()?.to_string_lossy().into_owned().into(),
                        )?;
                        js_sys::Reflect::set(
                            &obj,
                            &"contents".into(),
                            &ReadableStream::from_async_read(entry, 1024)
                                .into_raw()
                                .into(),
                        )?;
                        Ok(obj.into())
                    },
                )
                .map_err(|e| e.into())
        });
        Ok(ReadableStream::from_stream(entries).into_raw())
    }
}
