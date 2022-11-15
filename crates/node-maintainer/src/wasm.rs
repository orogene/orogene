use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;
use url::Url;
use wasm_bindgen::prelude::*;

use crate::{NodeMaintainer, NodeMaintainerError, NodeMaintainerOptions};

type Result<T> = std::result::Result<T, JsNodeMaintainerError>;

#[derive(Error, Debug)]
#[error("{0}")]
#[wasm_bindgen(js_name = NodeMaintainerError)]
pub struct JsNodeMaintainerError(#[from] NodeMaintainerError);

#[derive(Debug, Deserialize)]
#[wasm_bindgen(js_name = NodeMaintainerOptions)]
pub struct JsNodeMaintainerOptions {
    registry: Option<Url>,
    scoped_registries: Option<HashMap<String, Url>>,
}

#[wasm_bindgen(js_name = NodeMaintainer)]
pub struct JsNodeMaintainer(NodeMaintainer);

#[wasm_bindgen(js_class = NodeMaintainer)]
impl JsNodeMaintainer {
    pub fn to_kdl(&self) -> String {
        self.0.to_kdl().to_string()
    }
}

#[wasm_bindgen(variadic)]
pub async fn resolve(spec: &str, opts: JsValue) -> Result<JsNodeMaintainer> {
    console_error_panic_hook::set_once();
    let mut opts_builder = NodeMaintainerOptions::new();
    if opts.is_object() {
        let opts: JsNodeMaintainerOptions = serde_wasm_bindgen::from_value(opts)
            .map_err(|e| JsNodeMaintainerError(NodeMaintainerError::MiscError(format!("{e}"))))?;
        if let Some(registry) = opts.registry {
            opts_builder = opts_builder.registry(registry);
        }
        if let Some(scopes) = opts.scoped_registries {
            for (scope, registry) in scopes {
                opts_builder = opts_builder.scope_registry(scope, registry);
            }
        }
    }
    opts_builder
        .resolve(spec)
        .await
        .map(JsNodeMaintainer)
        .map_err(JsNodeMaintainerError)
}
