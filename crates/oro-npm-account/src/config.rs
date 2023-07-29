use base64::{engine::general_purpose, Engine as _};
use kdl::{KdlDocument, KdlNode, KdlValue};

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
            if children.get_mut("user").is_none() {
                children.nodes_mut().push(KdlNode::new("user"));
                children.get_mut("user").unwrap().ensure_children();
            }
        }
    }

    if let Some(user) = config
        .get_mut("options")
        .unwrap()
        .children_mut()
        .as_mut()
        .unwrap()
        .get_mut("user")
        .unwrap()
        .children_mut()
    {
        match credentials {
            Credentials::AuthToken(auth_token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(format!("{uri}:_authToken"));
                clean_nodes(uri, current_node);
                node.push(KdlValue::String(auth_token.to_owned()));
                current_node.push(node);
            }
            Credentials::Auth(token) => {
                let current_node = user.nodes_mut();
                let mut node = KdlNode::new(format!("{uri}:_auth"));
                clean_nodes(uri, current_node);
                node.push(KdlValue::String(token.to_owned()));
                current_node.push(node);
            }
            Credentials::UsernameAndPassword { username, password } => {
                let current_node = user.nodes_mut();
                let mut username_node = KdlNode::new(format!("{uri}:username"));
                let mut password_node = KdlNode::new(format!("{uri}:password"));
                clean_nodes(uri, current_node);
                username_node.push(KdlValue::String(username.to_owned()));
                password_node.push(KdlValue::String(general_purpose::STANDARD.encode(password)));
                current_node.push(username_node);
                current_node.push(password_node);
            }
        }
    }
}

pub fn get_credentials_by_uri(uri: &str, config: &KdlDocument) -> Option<Credentials> {
    config
        .get("options")
        .and_then(|options| options.children())
        .and_then(|options_children| options_children.get("user"))
        .and_then(|user| user.children())
        .and_then(|user_children| {
            let token = user_children.get(&format!("{uri}:_auth"));
            let auth_token = user_children.get(&format!("{uri}:_authToken"));
            let username = user_children.get(&format!("{uri}:username"));
            let password = user_children.get(&format!("{uri}:_password"));

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
    nodes.retain_mut(|node| {
        !(node.name().value() == format!("{uri}:_authToken")
            || node.name().value() == format!("{uri}:_auth")
            || node.name().value() == format!("{uri}:username")
            || node.name().value() == format!("{uri}:_password"))
    });
}

fn extract_string(input: &KdlNode) -> String {
    input
        .entries()
        .last()
        .unwrap()
        .value()
        .as_string()
        .unwrap()
        .into()
}
