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
                clean_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("auth-token", auth_token.as_ref()));
                current_node.push(node);
            }
            Credentials::Auth(token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri);
                clean_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("auth", token.as_ref()));
                current_node.push(node);
            }
            Credentials::UsernameAndPassword { username, password } => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri);
                clean_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("username", username.as_ref()));
                node.push(KdlEntry::new_prop("password", password.as_ref()));
                current_node.push(node);
            }
        }
    }
}

pub fn get_credentials_by_uri(uri: &str, config: &KdlDocument) -> Option<Credentials> {
    config
        .get("options")
        .and_then(|options| options.children())
        .and_then(|options_children| options_children.get("auth"))
        .and_then(|user| user.children())
        .and_then(|user_children| user_children.get(uri))
        .and_then(|user_children| {
            let token = user_children.get("auth");
            let auth_token = user_children.get("auth-token");
            let username = user_children.get("username");
            let password = user_children.get("password");

            match (token, auth_token, username, password) {
                (.., Some(username), Some(password)) => {
                    let username = extract_string(username);
                    let password = extract_string(password);
                    let password = general_purpose::STANDARD.decode(password).unwrap();
                    let password = String::from_utf8_lossy(&password).to_string();

                    Some(Credentials::Auth(
                        general_purpose::STANDARD.encode(format!("{username}:{password}")),
                    ))
                }
                (_, Some(auth_token), ..) => {
                    Some(Credentials::AuthToken(extract_string(auth_token)))
                }
                (Some(token), ..) => Some(Credentials::Auth(extract_string(token))),
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
        clean_nodes(uri, user_children.nodes_mut());
    };
}

fn clean_nodes(uri: &str, nodes: &mut Vec<KdlNode>) {
    nodes.retain_mut(|node| node.name().value() != uri);
}

fn extract_string(input: &KdlValue) -> String {
    input.as_string().unwrap().into()
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
