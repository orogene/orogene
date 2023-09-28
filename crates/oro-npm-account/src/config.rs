use base64::{engine::general_purpose, Engine as _};
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use reqwest::header::HeaderValue;

pub enum Credentials {
    AuthToken(String),
    /// Decryptable username and password combinations
    Auth(String),
    UsernameAndPassword {
        username: String,
        password: String,
    },
}

pub fn set_credentials_by_uri(uri: &str, credentials: &Credentials, config: &mut KdlDocument) {
    if config.get_mut("options").is_none() {
        config.nodes_mut().push(KdlNode::new("options"));
    }
    if let Some(opts) = config.get_mut("options") {
        opts.ensure_children();
        if let Some(children) = opts.children_mut().as_mut() {
            if children.get_mut("auth").is_none() {
                children.nodes_mut().push(KdlNode::new("auth"));
                children.get_mut("auth").unwrap().ensure_children();
            }
        }
    }

    if let Some(user) = config
        .get_mut("options")
        .and_then(|options| options.children_mut().as_mut())
        .and_then(|options_children| options_children.get_mut("auth"))
        .and_then(|user| user.children_mut().as_mut())
    {
        match credentials {
            Credentials::AuthToken(auth_token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri);
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("auth-token", auth_token.as_ref()));
                current_node.push(node);
            }
            Credentials::Auth(token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri);
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("auth", token.as_ref()));
                current_node.push(node);
            }
            Credentials::UsernameAndPassword { username, password } => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri);
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("username", username.as_ref()));
                node.push(KdlEntry::new_prop("password", password.as_ref()));
                current_node.push(node);
            }
        }
    }
}

pub fn set_scoped_registry(scope: &str, registry: &str, config: &mut KdlDocument) {
    if config.get_mut("options").is_none() {
        config.nodes_mut().push(KdlNode::new("options"));
    }
    if let Some(opts) = config.get_mut("options") {
        opts.ensure_children();
        if let Some(children) = opts.children_mut().as_mut() {
            if children.get_mut("scoped-registries").is_none() {
                children.nodes_mut().push(KdlNode::new("scoped-registries"));
                children
                    .get_mut("scoped-registries")
                    .unwrap()
                    .ensure_children();
            }
        }
    }

    if let Some(scoped_registries) = config
        .get_mut("options")
        .and_then(|options| options.children_mut().as_mut())
        .and_then(|options_children| options_children.get_mut("scoped-registries"))
        .and_then(|scoped_registries| scoped_registries.children_mut().as_mut())
    {
        let current_node = scoped_registries.nodes_mut();
        let mut node = KdlNode::new(scope);
        clean_scoped_registry_nodes(scope, current_node);
        node.push(KdlValue::String(registry.to_owned()));
        current_node.push(node);
    }
}

pub fn get_credentials_by_uri(uri: &str, config: &KdlDocument) -> Option<Credentials> {
    config
        .get("options")
        .and_then(|options| options.children())
        .and_then(|options_children| options_children.get("auth"))
        .and_then(|user| user.children())
        .and_then(|user_children| user_children.get(uri))
        .and_then(|credentials| {
            let token = credentials.get("auth");
            let auth_token = credentials.get("auth-token");
            let username = credentials.get("username");
            let password = credentials.get("password");

            match (token, auth_token, username, password) {
                (.., Some(username), Some(password)) => {
                    let username = username.as_string()?;
                    let password = password.as_string()?;
                    let password = general_purpose::STANDARD.decode(password).ok()?;
                    let password = String::from_utf8_lossy(&password).to_string();

                    Some(Credentials::Auth(
                        general_purpose::STANDARD.encode(format!("{username}:{password}")),
                    ))
                }
                (_, Some(auth_token), ..) => {
                    Some(Credentials::AuthToken(auth_token.as_string()?.into()))
                }
                (Some(token), ..) => Some(Credentials::Auth(token.as_string()?.into())),
                _ => None,
            }
        })
}

pub fn clear_crendentials_by_uri(uri: &str, config: &mut KdlDocument) {
    if let Some(user_children) = config
        .get_mut("options")
        .and_then(|options| options.children_mut().as_mut())
        .and_then(|options_children| options_children.get_mut("user"))
        .and_then(|user| user.children_mut().as_mut())
    {
        clean_auth_nodes(uri, user_children.nodes_mut());
    };
}

fn clean_auth_nodes(uri: &str, nodes: &mut Vec<KdlNode>) {
    nodes.retain_mut(|node| node.name().value() != uri);
}

fn clean_scoped_registry_nodes(scope: &str, nodes: &mut Vec<KdlNode>) {
    nodes.retain_mut(|node| node.name().value() != scope);
}

impl TryFrom<Credentials> for HeaderValue {
    type Error = crate::error::OroNpmAccountError;

    fn try_from(value: Credentials) -> Result<Self, Self::Error> {
        match value {
            Credentials::AuthToken(auth_token) => {
                Ok(HeaderValue::from_str(&format!("Bearer {auth_token}"))?)
            }
            Credentials::Auth(auth) => Ok(HeaderValue::from_str(&format!("Basic {auth}"))?),
            _ => Err(Self::Error::UnsupportedConversionError),
        }
    }
}
