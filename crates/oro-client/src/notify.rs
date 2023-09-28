use reqwest::Response;

pub(crate) trait Notify {
    fn notify(self) -> Self;
}

impl Notify for Response {
    fn notify(self) -> Self {
        if let Some(npm_notice) = self.headers().get("npm-notice") {
            if let Ok(npm_notice) = npm_notice.to_str() {
                tracing::info!("{}", npm_notice);
            }
        }
        self
    }
}
