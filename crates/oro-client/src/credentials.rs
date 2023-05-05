use std::{collections::HashMap, fmt::Debug};

use crate::OroClientError;
/**
 * Different credential types supported by orogene
 */
#[derive(Clone)]
pub enum Credentials {
    /// HTTP basic auth credentials
    Basic { username: String, password: String },
    /// HTTP Bearer token auth
    Token(String),
}

impl Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic { username, password } => {
                f.write_fmt(format_args!("Basic(username={},password=***)", username))
            }
            Self::Token(string) => f.write_str("Token(***)"),
        }
    }
}

impl TryFrom<HashMap<String, String>> for Credentials {
    type Error = OroClientError;

    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        // TODO I don't like those &...to_string() constructs -> find a better type for the HashMap!
        if let Some(token) = value.get("token") {
            Ok(Self::Token(token.to_owned()))
        } else if let (Some(username), Some(password)) =
            (value.get("username"), value.get("password"))
        {
            Ok(Self::Basic {
                username: username.to_owned(),
                password: password.to_owned(),
            })
        } else {
            Err(OroClientError::CredentialsConfigError(
                "Credentials either have to contain a token or username/password".to_string(),
            ))
        }
    }
}
