use crate::error::OroNpmAccountError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password};
use open::that as open;
use oro_client::login::{AuthType, DoneURLResponse, LoginCouchResponse, LoginOptions, Token};
use oro_client::OroClient;
use url::Url;

pub async fn login(
    auth_type: &AuthType,
    registry: &Url,
    options: &LoginOptions,
) -> Result<Token, OroNpmAccountError> {
    let client = options
        .client
        .clone()
        .unwrap_or_else(|| OroClient::new(registry.clone()));
    match auth_type {
        AuthType::Web => {
            let login_web = client.login_web(options).await?;
            // TODO: make clickable in supported terminals.
            tracing::info!("Login URL: {}", login_web.login_url);
            open(login_web.login_url).map_err(OroNpmAccountError::OpenURLError)?;

            loop {
                match client.fetch_done_url(&login_web.done_url).await? {
                    DoneURLResponse::Token(token) => break Ok(Token { token }),
                    DoneURLResponse::Duration(duration) => {
                        async_std::task::sleep(duration).await;
                    }
                }
            }
        }
        AuthType::Legacy => {
            let username: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Username:")
                .interact()
                .map_err(OroNpmAccountError::ReadUserInputError)?;
            let password: String = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Password:")
                .interact()
                .map_err(OroNpmAccountError::ReadUserInputError)?;

            match client
                .login_couch(&username, &password, None, options)
                .await?
            {
                LoginCouchResponse::WebOTP { auth_url, done_url } => {
                    open(auth_url).map_err(OroNpmAccountError::OpenURLError)?;

                    loop {
                        match client.fetch_done_url(&done_url).await? {
                            DoneURLResponse::Token(token) => break Ok(Token { token }),
                            DoneURLResponse::Duration(duration) => {
                                async_std::task::sleep(duration).await;
                            }
                        }
                    }
                }
                LoginCouchResponse::ClassicOTP => {
                    let otp: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("This operation requires a one-time password. Enter OTP:")
                        .interact()
                        .map_err(OroNpmAccountError::ReadUserInputError)?;

                    match client
                        .login_couch(&username, &password, Some(&otp), options)
                        .await?
                    {
                        LoginCouchResponse::Token(token) => Ok(Token { token }),
                        _ => Err(OroNpmAccountError::UnexpectedResponseError),
                    }
                }
                LoginCouchResponse::Token(token) => Ok(Token { token }),
            }
        }
    }
}
