use reqwest::Response;
use reqwest_middleware::RequestBuilder;

pub(crate) trait Notify {
    fn notify(self) -> Self;
}

pub(crate) trait Otp {
    fn otp(self, otp: Option<&str>) -> Self;
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

impl Otp for RequestBuilder {
    fn otp(self, otp: Option<&str>) -> Self {
        if let Some(otp) = otp {
            return self.header("npm-otp", otp);
        }
        self
    }
}
