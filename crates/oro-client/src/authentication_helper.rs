use crate::error::OroClientError;
use reqwest::header::WWW_AUTHENTICATE;
use reqwest::Response;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct WebOTPResponse {
    auth_url: Option<String>,
    done_url: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OTPResponse {
    WebOTP { auth_url: String, done_url: String },
    ClassicOTP,
}

pub struct AuthenticationHelper;

impl AuthenticationHelper {
    pub async fn check_response(
        response: Response,
        otp: Option<&str>,
    ) -> Result<OTPResponse, OroClientError> {
        let www_authenticate = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .map_or(String::default(), |header| {
                header.to_str().unwrap().to_lowercase()
            });

        let text = response.text().await?;
        let json = serde_json::from_str::<WebOTPResponse>(&text).unwrap_or_default();

        if www_authenticate.contains("otp") || text.to_lowercase().contains("one-time pass") {
            if otp.is_none() {
                if let (Some(auth_url), Some(done_url)) = (json.auth_url, json.done_url) {
                    Ok(OTPResponse::WebOTP { auth_url, done_url })
                } else {
                    Ok(OTPResponse::ClassicOTP)
                }
            } else {
                Err(OroClientError::OTPRequiredError)
            }
        } else {
            Err(if www_authenticate.contains("basic") {
                OroClientError::IncorrectPasswordError
            } else if www_authenticate.contains("bearer") {
                OroClientError::InvalidTokenError
            } else {
                OroClientError::UnauthorizedError
            })
        }
    }
}
