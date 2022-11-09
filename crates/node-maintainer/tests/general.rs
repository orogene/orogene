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
    setup_packuments(&mock_server).await;
    let nm = NodeMaintainer::builder()
        .registry(mock_server.uri().parse().into_diagnostic()?)
        .resolve("a")
        .await?;

    println!("{}", nm.render());

    Ok(())
}

async fn setup_packuments(mock_server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "versions": {
                "1.0.0": {
                    "name": "a",
                    "version": "1.0.0",
                    "dependencies": {
                        "b": "2.0.0"
                    },
                    "dist": {
                        "tarball": "https://example.com/a-1.0.0.tgz"
                    }
                }
            },
            "dist-tags": {
                "latest": "1.0.0"
            }
        })))
        .mount(mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "versions": {
                "2.0.0": {
                    "name": "b",
                    "version": "2.0.0",
                    "dependencies": {
                        "c": "3.0.0"
                    },
                    "dist": {
                        "tarball": "https://example.com/b-2.0.0.tgz"
                    }
                }
            },
            "dist-tags": {
                "latest": "2.0.0"
            }
        })))
        .mount(mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("c"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "versions": {
                "3.0.0": {
                    "name": "c",
                    "version": "3.0.0",
                    "dist": {
                        "tarball": "https://example.com/c-3.0.0.tgz"
                    }
                }
            },
            "dist-tags": {
                "latest": "3.0.0"
            }
        })))
        .mount(mock_server)
        .await;
}
