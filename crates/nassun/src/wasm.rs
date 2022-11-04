//! WASM-oriented Nassun interface for more idiomatic JS usage.

use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;
use url::Url;
use wasm_bindgen::prelude::*;

use crate::{Nassun, NassunError, NassunOpts};

type Result<T> = std::result::Result<T, JsNassunError>;

#[wasm_bindgen(js_name = NassunError)]
#[derive(Error, Debug)]
#[error("{0}")]
pub struct JsNassunError(#[from] NassunError);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsNassunOpts {
    use_corgi: Option<bool>,
    registry: Option<Url>,
    scoped_registries: Option<HashMap<String, Url>>,
}

#[wasm_bindgen(js_name = Nassun)]
pub struct JsNassun(Nassun);

#[wasm_bindgen]
impl JsNassun {
    #[wasm_bindgen(constructor, variadic)]
    pub fn new(mut args: Vec<JsValue>) -> Result<JsNassun> {
        if let Some(opts) = args.pop() {
            let mut opts_builder = NassunOpts::new();
            let opts: JsNassunOpts = serde_wasm_bindgen::from_value(opts)
                .map_err(|e| NassunError::MiscError(format!("{}", e)))?;
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
}
