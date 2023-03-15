use std::collections::HashMap;

use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use node_maintainer::NodeMaintainer;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[async_std::test]
async fn basic_flatten() -> Result<()> {
    let mock_server = MockServer::start().await;
    // This tests a basic linear dependency chain with no conflicts flattens
    // completely: a -> b -> c -> d
    let mock_data = r#"
    a {
        version "1.0.0"
        dependencies {
            b "^2.0.0"
        }
    }
    b {
        version "2.0.0"
        dependencies {
            c "^3.0.0"
        }
    }
    c {
        version "3.0.0"
        dependencies {
            d "^4.0.0"
        }
    }
    d {
        version "4.0.0"
    }
    "#;
    mocks_from_kdl(&mock_server, mock_data.parse()?).await;
    let nm = NodeMaintainer::builder()
        .concurrency(1)
        .registry(mock_server.uri().parse().into_diagnostic()?)
        .resolve_spec("a@^1")
        .await?;

    assert_eq!(
        nm.to_kdl()?.to_string(),
        r#"// This file is automatically generated and not intended for manual editing.
lockfile-version 1
root "a" {
    version "1.0.0"
    dependencies {
        b ">=2.0.0 <3.0.0-0"
    }
}
pkg "b" {
    version "2.0.0"
    resolved "https://example.com/-/b-2.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        c ">=3.0.0 <4.0.0-0"
    }
}
pkg "c" {
    version "3.0.0"
    resolved "https://example.com/-/c-3.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        d ">=4.0.0 <5.0.0-0"
    }
}
pkg "d" {
    version "4.0.0"
    resolved "https://example.com/-/d-4.0.0.tgz"
    integrity "sha512-deadbeef"
}
"#
    );
    Ok(())
}

#[async_std::test]
async fn nesting_simple_conflict() -> Result<()> {
    let mock_server = MockServer::start().await;
    // Testing that simple conflicts get resolved correctly.
    let mock_data = r#"
    a {
        version "1.0.0"
        dependencies {
            b "^2.0.0"
        }
    }
    b {
        version "2.0.0"
        dependencies {
            c "^3.0.0"
            d "^4.0.0"
        }
    }
    c {
        version "3.0.0"
    }
    c {
        version "5.0.0"
    }
    d {
        version "4.0.0"
        dependencies {
            // This one will conflict with the `c@3.0.0` already placed in the
            // root, so it should be nested under `d`
            c "^5.0.0"
        }
    }
    "#;
    mocks_from_kdl(&mock_server, mock_data.parse()?).await;
    let nm = NodeMaintainer::builder()
        .concurrency(1)
        .registry(mock_server.uri().parse().into_diagnostic()?)
        .resolve_spec("a@^1")
        .await?;

    assert_eq!(
        nm.to_kdl()?.to_string(),
        r#"// This file is automatically generated and not intended for manual editing.
lockfile-version 1
root "a" {
    version "1.0.0"
    dependencies {
        b ">=2.0.0 <3.0.0-0"
    }
}
pkg "b" {
    version "2.0.0"
    resolved "https://example.com/-/b-2.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        c ">=3.0.0 <4.0.0-0"
        d ">=4.0.0 <5.0.0-0"
    }
}
pkg "c" {
    version "3.0.0"
    resolved "https://example.com/-/c-3.0.0.tgz"
    integrity "sha512-deadbeef"
}
pkg "d" {
    version "4.0.0"
    resolved "https://example.com/-/d-4.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        c ">=5.0.0 <6.0.0-0"
    }
}
pkg "d" "c" {
    version "5.0.0"
    resolved "https://example.com/-/c-5.0.0.tgz"
    integrity "sha512-deadbeef"
}
"#
    );
    Ok(())
}

#[async_std::test]
async fn nesting_sibling_conflict() -> Result<()> {
    let mock_server = MockServer::start().await;
    // This tests that when a dependency conflict comes from different
    // branches of a tree, the "phantom" hoisted dependency is correctly
    // detected, and the one we were trying to bubble up is correctly nested.
    let mock_data = r#"
    a {
        version "1.0.0"
        dependencies {
            b "^2.0.0"
            c "^3.0.0"
        }
    }
    b {
        version "2.0.0"
        dependencies {
            d "^4.0.0"
        }
    }
    c {
        version "3.0.0"
        dependencies {
            d "^5.0.0"
        }
    }
    d {
        version "4.0.0"
    }
    d {
        version "5.0.0"
    }
    "#;
    mocks_from_kdl(&mock_server, mock_data.parse()?).await;
    let nm = NodeMaintainer::builder()
        .concurrency(1)
        .registry(mock_server.uri().parse().into_diagnostic()?)
        .resolve_spec("a@^1")
        .await?;

    assert_eq!(
        nm.to_kdl()?.to_string(),
        r#"// This file is automatically generated and not intended for manual editing.
lockfile-version 1
root "a" {
    version "1.0.0"
    dependencies {
        b ">=2.0.0 <3.0.0-0"
        c ">=3.0.0 <4.0.0-0"
    }
}
pkg "b" {
    version "2.0.0"
    resolved "https://example.com/-/b-2.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        d ">=4.0.0 <5.0.0-0"
    }
}
pkg "c" {
    version "3.0.0"
    resolved "https://example.com/-/c-3.0.0.tgz"
    integrity "sha512-deadbeef"
    dependencies {
        d ">=5.0.0 <6.0.0-0"
    }
}
pkg "c" "d" {
    version "5.0.0"
    resolved "https://example.com/-/d-5.0.0.tgz"
    integrity "sha512-deadbeef"
}
pkg "d" {
    version "4.0.0"
    resolved "https://example.com/-/d-4.0.0.tgz"
    integrity "sha512-deadbeef"
}
"#
    );
    Ok(())
}

async fn mocks_from_kdl(mock_server: &MockServer, doc: KdlDocument) {
    let mut packuments = HashMap::new();
    for node in doc.nodes() {
        let name = node.name().value().to_owned();
        let children = node.children().unwrap();
        let version = children
            .get_arg("version")
            .unwrap()
            .as_string()
            .unwrap()
            .to_owned();
        let dependencies = children.get("dependencies").map(|deps| {
            let dep_kids = deps.children().unwrap();
            let mut deps = json!({});
            for dep in dep_kids.nodes() {
                deps[dep.name().to_string()] = json!(dep.get(0).unwrap().as_string().unwrap());
            }
            deps
        });
        let packument = packuments.entry(name.clone()).or_insert_with(|| {
            json!({
                "versions": {},
                "dist-tags": {}
            })
        });
        packument["versions"][version.clone()] = json!({
            "name": name.clone(),
            "version": version.clone(),
            "dist": {
                "tarball": format!("https://example.com/-/{name}-{version}.tgz"),
                "integrity": "sha512-deadbeef"
            }
        });
        if let Some(deps) = dependencies {
            packument["versions"][version.clone()]["dependencies"] = deps;
        }
        // Last version gets "latest"
        packument["dist-tags"]["latest"] = json!(version);
    }

    for (name, packument) in packuments {
        Mock::given(method("GET"))
            .and(path(name))
            .respond_with(ResponseTemplate::new(200).set_body_json(&packument))
            .mount(mock_server)
            .await;
    }
}
