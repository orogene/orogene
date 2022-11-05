//! WASM-oriented Nassun interface for more idiomatic JS usage.

use std::collections::HashMap;

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use wasm_bindgen::prelude::*;

use crate::{Nassun, NassunError, NassunOpts, Package};

type Result<T> = std::result::Result<T, JsNassunError>;

#[wasm_bindgen]
pub async fn metadata(spec: &str, opts: JsValue) -> Result<JsValue> {
    JsNassun::new(opts)?.resolve(spec).await?.metadata().await
}

#[wasm_bindgen(js_name = NassunError)]
#[derive(Error, Debug)]
#[error("{0}")]
pub struct JsNassunError(#[from] NassunError);

#[wasm_bindgen(js_class = NassunError)]
impl JsNassunError {
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> Option<String> {
        self.0.code().map(|c| c.to_string())
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_js_string(&self) -> String {
        format!(
            "JsNasunError({}: {})",
            self.0
                .code()
                .unwrap_or_else(|| Box::new("nassun::code_unavailable")),
            self.0
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsNassunOpts {
    use_corgi: Option<bool>,
    registry: Option<Url>,
    scoped_registries: Option<HashMap<String, Url>>,
}

#[wasm_bindgen(js_name = Nassun)]
pub struct JsNassun(Nassun);

#[wasm_bindgen(js_class = Nassun)]
impl JsNassun {
    #[wasm_bindgen(constructor, variadic)]
    pub fn new(opts: JsValue) -> Result<JsNassun> {
        if opts.is_object() {
            let mut opts_builder = NassunOpts::new();
            let opts: JsNassunOpts = serde_wasm_bindgen::from_value(opts)
                .map_err(|e| JsNassunError(NassunError::MiscError(format!("{e}"))))?;
            if let Some(use_corgi) = opts.use_corgi {
                opts_builder = opts_builder.use_corgi(use_corgi);
            }
            if let Some(registry) = opts.registry {
                opts_builder = opts_builder.registry(registry);
            }
            if let Some(scopes) = opts.scoped_registries {
                for (scope, registry) in scopes {
                    opts_builder = opts_builder.scope_registry(scope, registry);
                }
            }
            Ok(JsNassun(opts_builder.build()))
        } else {
            Ok(JsNassun(Nassun::new()))
        }
    }

    pub async fn resolve(&self, spec: &str) -> Result<JsPackage> {
        let package = self.0.resolve(spec).await?;
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        Ok(JsPackage {
            from: JsValue::from_str(&package.from().to_string()),
            name: JsValue::from_str(package.name()),
            resolved: JsValue::from_str(&format!("{}", package.resolved())),
            package,
            serializer,
        })
    }
}

#[wasm_bindgen(js_name = Package)]
pub struct JsPackage {
    from: JsValue,
    name: JsValue,
    resolved: JsValue,
    package: Package,
    serializer: serde_wasm_bindgen::Serializer,
}

#[wasm_bindgen(js_class = Package)]
impl JsPackage {
    #[wasm_bindgen(getter)]
    pub fn from(&self) -> JsValue {
        self.from.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> JsValue {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn resolved(&self) -> JsValue {
        self.resolved.clone()
    }

    pub fn packument(&self) -> Result<JsValue> {
        self.package
            .packument()
            .serialize(&self.serializer)
            .map_err(|e| JsNassunError(NassunError::MiscError(format!("{e}"))))
    }

    pub async fn metadata(&self) -> Result<JsValue> {
        self.package
            .metadata()
            .await
            .map_err(|e| JsNassunError(NassunError::MiscError(format!("{e}"))))?
            .serialize(&self.serializer)
            .map_err(|e| JsNassunError(NassunError::MiscError(format!("{e}"))))
    }
}
