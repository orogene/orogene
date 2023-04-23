use std::{collections::HashMap, fmt::Debug};

use crate::OroClientError;

/**
 * HTTP basic credentials (i.e. username & password)
 */
#[derive(Clone)]
pub struct BasicAuthCredentials {
    pub username: String,
    pub password: String,
}

impl Debug for BasicAuthCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("username: {}, password: ***", self.username))
    }
}

/**
 * Different credential types supported by orogene
 */
#[derive(Clone, Debug)]
pub enum Credentials {
    Basic(BasicAuthCredentials),
    Token(String),
}

impl TryFrom<&HashMap<&String, &String>> for Credentials {
    type Error = OroClientError;

    fn try_from(value: &HashMap<&String, &String>) -> Result<Self, Self::Error> {
        // TODO I don't like those &...to_string() constructs -> find a better type for the HashMap!
        if let Some(&token) = value.get(&"token".to_string()) {
            Ok(Self::Token(token.to_owned()))
        } else if value.contains_key(&"username".to_owned())
            && value.contains_key(&"password".to_string())
        {
            Ok(Self::Basic(BasicAuthCredentials {
                username: value.get(&"username".to_string()).unwrap().to_string(),
                password: value.get(&"password".to_string()).unwrap().to_string(),
            }))
        } else {
            Err(OroClientError::CredentialsConfigError(
                "Credentials either have to contain a token or username/password".to_string(),
            ))
        }
    }
}
