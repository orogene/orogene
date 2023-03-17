use std::{collections::HashMap, path::Path};

use miette::Diagnostic;
use nassun::Package;
use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::error::NodeMaintainerError;

type Result<T> = std::result::Result<T, NodeMaintainerError>;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export interface NodeMaintainerError {
    message: string;
    code?: string;
}

export interface PackageJson {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    bundledDependencies?: string[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PackageJson")]
    pub type PackageJson;
}

impl From<NodeMaintainerError> for JsValue {
    fn from(e: NodeMaintainerError) -> Self {
        let obj = js_sys::Object::new();
        let msg = format!("{e}");
        js_sys::Reflect::set(&obj, &"message".into(), &JsValue::from_str(&msg))
            .unwrap_or_else(|_| panic!("failed to set error message: {e}"));
        if let Some(code) = e.code() {
            let code = format!("{code}");
            js_sys::Reflect::set(&obj, &"code".into(), &JsValue::from_str(&code))
                .unwrap_or_else(|_| panic!("failed to set error code: {e:#?}"));
        }
        obj.into()
    }
}

/// Options for configuration for various `NodeMaintainer` operations.
#[derive(Tsify, Debug, Deserialize)]
#[allow(non_snake_case)]
#[wasm_bindgen]
pub struct NodeMaintainerOptions {
    #[tsify(optional)]
    registry: Option<String>,
    #[tsify(optional)]
    scopedRegistries: Option<HashMap<String, String>>,
    #[tsify(optional)]
    concurrency: Option<usize>,
    #[tsify(optional)]
    kdlLock: Option<String>,
    #[tsify(optional)]
    npmLock: Option<String>,
    #[tsify(optional)]
    defaultTag: Option<String>,
}

/// An NPM-compatible dependency resolver. NodeMaintainer builds trees of
/// package nodes that can be used to generate lockfiles or fetch package
/// tarballs, or even extract them to where they would live in `node_modules`.
#[derive(Tsify)]
#[wasm_bindgen]
pub struct NodeMaintainer {
    #[wasm_bindgen(skip)]
    pub inner: crate::maintainer::NodeMaintainer,
}

impl NodeMaintainer {
    fn new(inner: crate::maintainer::NodeMaintainer) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen]
impl NodeMaintainer {
    fn opts_from_js_value(opts: JsValue) -> Result<crate::maintainer::NodeMaintainerOptions> {
        console_error_panic_hook::set_once();
        let mut opts_builder = crate::maintainer::NodeMaintainer::builder();
        let opts: Option<NodeMaintainerOptions> = serde_wasm_bindgen::from_value(opts)?;
        if let Some(opts) = opts {
            if let Some(registry) = opts.registry {
                opts_builder = opts_builder.registry(
                    registry
                        .parse()
                        .map_err(|e| NodeMaintainerError::UrlParseError(registry, e))?,
                );
            }
            if let Some(scopes) = opts.scopedRegistries {
                for (scope, registry) in scopes {
                    opts_builder = opts_builder.scope_registry(
                        scope,
                        registry
                            .parse()
                            .map_err(|e| NodeMaintainerError::UrlParseError(registry, e))?,
                    );
                }
            }
            if let Some(concurrency) = opts.concurrency {
                opts_builder = opts_builder.concurrency(concurrency);
            }
            if let Some(kdl_lock) = opts.kdlLock {
                opts_builder = opts_builder.kdl_lock(kdl_lock)?;
            }
            if let Some(npm_lock) = opts.npmLock {
                opts_builder = opts_builder.npm_lock(npm_lock)?;
            }
            if let Some(default_tag) = opts.defaultTag {
                opts_builder = opts_builder.default_tag(default_tag);
            }
        }
        Ok(opts_builder)
    }

    /// Resolves a dependency tree using `spec` as the root package.
    #[wasm_bindgen(js_name = "resolveSpec")]
    pub async fn resolve_spec(spec: &str, opts: JsValue) -> Result<NodeMaintainer> {
        console_error_panic_hook::set_once();
        let opts_builder = Self::opts_from_js_value(opts)?;
        opts_builder
            .resolve_spec(spec)
            .await
            .map(NodeMaintainer::new)
    }

    /// Returns a dependency tree using a `package.json` manifest as the root
    /// package.
    #[wasm_bindgen(js_name = "resolveManifest")]
    pub async fn resolve_manifest(manifest: PackageJson, opts: JsValue) -> Result<NodeMaintainer> {
        console_error_panic_hook::set_once();
        let manifest = serde_wasm_bindgen::from_value(manifest.into())?;
        let opts_builder = Self::opts_from_js_value(opts)?;
        opts_builder
            .resolve_manifest(manifest)
            .await
            .map(NodeMaintainer::new)
    }

    /// Returns the contents of a package-lock.kdl lockfile for this resolved tree.
    #[wasm_bindgen(js_name = "toKdl")]
    pub fn to_kdl(&self) -> Result<String> {
        Ok(self.inner.to_kdl()?.to_string())
    }

    /// Given a path within node_modules, returns the package that the
    /// referenced file/directory belongs to.
    #[wasm_bindgen(js_name = "packageAtPath")]
    pub fn package_at_path(&self, path: &str) -> Option<Package> {
        self.inner
            .package_at_path(Path::new(path))
            .map(Package::from_core_package)
    }
}

/// Resolves a dependency tree using `spec` as the root package.
#[wasm_bindgen(js_name = "resolveSpec")]
pub async fn resolve_spec(spec: &str, opts: JsValue) -> Result<NodeMaintainer> {
    NodeMaintainer::resolve_spec(spec, opts).await
}

/// Returns a dependency tree using a `package.json` manifest as the root
/// package.
#[wasm_bindgen(js_name = "resolveManifest")]
pub async fn resolve_manifest(manifest: PackageJson, opts: JsValue) -> Result<NodeMaintainer> {
    NodeMaintainer::resolve_manifest(manifest, opts).await
}
