#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
use anyhow::Result;
use orogene::Orogene;
use serde::{Deserialize, Serialize};

mod cmd;

// this really needs to goe somewhere else!
#[derive(Serialize)]
struct Response{
  msg: String,
}
#[async_std::main]
async fn main() -> Result<()> {
  tauri::AppBuilder::new()
    .invoke_handler(|_webview, arg| {
      use cmd::Cmd::*;
      match serde_json::from_str(arg) {
        Err(e) => {
          Err(e.to_string())
        }
        Ok(command) => {
          match command {
            // definitions for your custom commands from Cmd here
            BlockingEcho { msg } => {
              println!("UI is blocked while we print from rust: {}", msg);
            },
            AsyncEcho { callback, error, msg } => tauri::execute_promise(
              _webview,
              move || {
                println!("Async hello from rust {}", msg);
                let response = Response {
                  msg: format!("Modified by rust: {}", msg)
                };
                Ok(response)
              },
              callback,
              error,
            ),
          }
          Ok(())
        }
      }
    })
    .build()
    .run();
  Ok(())
}
