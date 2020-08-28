#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

use oro_config::OroConfigOptions;
use oro_gui_handler::OroHandler;

// NOTE: We need this because we need to make sure handlers get pulled in, so
// typetag can find them.
#[allow(unused_imports)]
use oro_handle_ping::PingHandler;

#[derive(Deserialize)]
struct Request {
    callback: String,
    error: String,
}

#[derive(Serialize)]
struct Response {
    body: Box<dyn erased_serde::Serialize>,
}

#[async_std::main]
async fn main() -> Result<()> {
    let config = OroConfigOptions::new().load()?;
    tauri::AppBuilder::new()
        .invoke_handler(move |_webview, arg| {
            let Request { callback, error } =
                serde_json::from_str(arg).map_err(|e| e.to_string())?;
            let cmd: Box<dyn OroHandler> = serde_json::from_str(arg).map_err(|e| e.to_string())?;
            // TODO: Arc/Mutex this? idk if I can
            let config = config.clone();
            tauri::execute_promise(
                _webview,
                move || {
                    async_std::task::block_on(async {
                        Ok(Response {
                            body: cmd.execute(&config).await?,
                        })
                    })
                },
                callback,
                error,
            );
            Ok(())
        })
        .build()
        .run();
    Ok(())
}
