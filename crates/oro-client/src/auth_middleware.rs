use std::{collections::HashMap, sync::Arc};

use reqwest::{header::HeaderValue, Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use task_local_extensions::Extensions;
use url::Url;

use crate::credentials::Credentials;

#[derive(Debug, Clone)]
pub(crate) struct AuthMiddleware(pub(crate) Arc<HashMap<String, Credentials>>);

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl Middleware for AuthMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let reg = Url::parse(
            req.headers()
                .get("X-Oro-Registry")
                .expect(
                    "Request did not have an x-oro-registry header. This is a bug in oro-client.",
                )
                .to_str()
                .expect("This should stringify just fine."),
        )
        .expect("This should have already been parsed and serialized previously.");

        // Don't add auth headers to requests to URLs outside the given
        // registry.
        let req_url = req.url().clone();
        if reg.host_str() != req_url.host_str() || !req_url.path().starts_with(reg.path()) {
            return next.run(req, extensions).await;
        }

        let credentials = self.0.get(&nerf_dart(&reg));
        if let Some(cred) = credentials {
            let auth_header = match cred {
                Credentials::Basic { username, password } => {
                    basic_auth(username, password.as_ref())
                }
                Credentials::EncodedBasic(auth) => {
                    let mut val = HeaderValue::from_str(&format!("Basic {auth}"))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    val.set_sensitive(true);
                    val
                }
                Credentials::Token(token) => {
                    let mut val = HeaderValue::from_str(&format!("Bearer {token}"))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    val.set_sensitive(true);
                    val
                }
            };
            req.headers_mut()
                .append(reqwest::header::AUTHORIZATION, auth_header);
        }
        next.run(req, extensions).await
    }
}

// From reqwest utils.
fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
where
    U: std::fmt::Display,
    P: std::fmt::Display,
{
    use base64::prelude::BASE64_STANDARD;
    use base64::write::EncoderWriter;
    use std::io::Write;

    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{}:", username);
        if let Some(password) = password {
            let _ = write!(encoder, "{}", password);
        }
    }
    let mut header = HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
    header.set_sensitive(true);
    header
}

pub fn nerf_dart(url: &Url) -> String {
    format!("//{}{}", url.host_str().unwrap_or(""), url.path())
}
