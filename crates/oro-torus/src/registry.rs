use oro_common::url::Url;

/// Configuration object for a particular registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Registry {
    pub scope: Option<String>,
    pub url: Url,
    pub auth: Option<AuthConfig>,
}

/// Authentication details for a registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthConfig {
    Token {
        token: String,
        always_auth: bool,
    },
    Credentials {
        username: String,
        password: String,
        always_auth: bool,
    },
}
