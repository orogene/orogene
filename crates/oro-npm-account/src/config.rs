use base64::{engine::general_purpose, Engine as _};
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use reqwest::header::HeaderValue;
use url::Url;

pub enum Credentials {
    Token(String),
    /// Decryptable username and password combinations
    LegacyAuth(String),
    BasicAuth {
        username: String,
        password: Option<String>,
    },
}

pub fn set_credentials_by_uri(uri: &Url, credentials: &Credentials, config: &mut KdlDocument) {
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
            Credentials::Token(auth_token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri.as_ref());
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("token", auth_token.as_ref()));
                current_node.push(node);
            }
            Credentials::LegacyAuth(token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri.as_ref());
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("legacy-auth", token.as_ref()));
                current_node.push(node);
            }
            Credentials::BasicAuth { username, password } => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(uri.as_ref());
                clean_auth_nodes(uri, current_node);
                node.push(KdlEntry::new_prop("username", username.as_ref()));
                if let Some(pass) = password {
                    node.push(KdlEntry::new_prop("password", pass.as_ref()));
                }
                current_node.push(node);
            }
        }
    }
}

pub fn set_scoped_registry(scope: &str, registry: &Url, config: &mut KdlDocument) {
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
        node.push(KdlValue::String(registry.as_ref().to_owned()));
        current_node.push(node);
    }
}

pub fn get_credentials_by_uri(uri: &Url, config: &KdlDocument) -> Option<Credentials> {
    config
        .get("options")
        .and_then(|options| options.children())
        .and_then(|options_children| options_children.get("auth"))
        .and_then(|user| user.children())
        .and_then(|user_children| {
            user_children.nodes().iter().find(|node| {
                let Ok(node_url) = Url::parse(node.name().value()) else {
                    return false;
                };
                oro_client::nerf_dart(&node_url) == oro_client::nerf_dart(uri)
            })
        })
        .and_then(|credentials| {
            let token = credentials.get("token");
            let legacy_auth = credentials.get("legacy-auth");
            let username = credentials.get("username");
            let password = credentials.get("password");

            match (token, legacy_auth, username, password) {
                (_, Some(token), ..) => Some(Credentials::Token(token.as_string()?.into())),
                (.., Some(username), Some(password)) => {
                    let username = username.as_string()?;
                    let password = password.as_string()?;
                    let password = general_purpose::STANDARD.decode(password).ok()?;
                    let password = String::from_utf8_lossy(&password).to_string();

                    Some(Credentials::LegacyAuth(
                        general_purpose::STANDARD.encode(format!("{username}:{password}")),
                    ))
                }
                (Some(legacy_auth), ..) => {
                    Some(Credentials::LegacyAuth(legacy_auth.as_string()?.into()))
                }
                _ => None,
            }
        })
}

pub fn clear_crendentials_by_uri(uri: &Url, config: &mut KdlDocument) {
    if let Some(auth_children) = config
        .get_mut("options")
        .and_then(|options| options.children_mut().as_mut())
        .and_then(|options_children| options_children.get_mut("auth"))
        .and_then(|auth| auth.children_mut().as_mut())
    {
        clean_auth_nodes(uri, auth_children.nodes_mut());
        if auth_children.nodes().is_empty() {
            if let Some(children) = config
                .get_mut("options")
                .and_then(|options| options.children_mut().as_mut())
            {
                children
                    .nodes_mut()
                    .retain_mut(|node| node.name().value() != "auth");
            }
        }
    };
}

fn clean_auth_nodes(uri: &Url, nodes: &mut Vec<KdlNode>) {
    nodes.retain_mut(|node| {
        let Ok(node_url) = Url::parse(node.name().value()) else {
            return false;
        };
        oro_client::nerf_dart(&node_url) != oro_client::nerf_dart(uri)
    });
}

fn clean_scoped_registry_nodes(scope: &str, nodes: &mut Vec<KdlNode>) {
    nodes.retain_mut(|node| node.name().value() != scope);
}

impl TryFrom<Credentials> for HeaderValue {
    type Error = crate::error::OroNpmAccountError;

    fn try_from(value: Credentials) -> Result<Self, Self::Error> {
        match value {
            Credentials::Token(auth_token) => {
                Ok(HeaderValue::from_str(&format!("Bearer {auth_token}"))?)
            }
            Credentials::LegacyAuth(auth) => Ok(HeaderValue::from_str(&format!("Basic {auth}"))?),
            _ => Err(Self::Error::UnsupportedConversionError),
        }
    }
}
