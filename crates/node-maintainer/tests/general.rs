use maplit::hashmap;
use miette::{IntoDiagnostic, Result};
use nassun::PackageResolution;
use node_maintainer::{NodeMaintainer, PackageNode, ResolvedTree};
use pretty_assertions::assert_eq;
use serde_json::json;
use unicase::UniCase;
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
        .resolve("b@^2")
        .await?;

    let expected = ResolvedTree {
        version: 1,
        root: pkg("b", "2", true, vec!["b".into()])?,
        packages: vec![pkg("c", "3", false, vec!["c".into()])?],
    };

    assert_eq!(expected, nm.to_resolved_tree());
    Ok(())
}

#[async_std::test]
async fn nesting_simple_conflict() -> Result<()> {
    let mock_server = MockServer::start().await;
    setup_packuments(&mock_server).await;
    let nm = NodeMaintainer::builder()
        .registry(mock_server.uri().parse().into_diagnostic()?)
        .resolve("a@^1")
        .await?;

    let expected = ResolvedTree {
        version: 1,
        root: pkg("a", "1", true, vec!["a".into()])?,
        packages: vec![
            pkg("b", "2", false, vec!["b".into()])?,
            pkg("c", "3", false, vec!["c".into()])?,
            pkg("d", "4", false, vec!["d".into()])?,
            pkg("c", "5", false, vec!["d".into(), "c".into()])?,
        ],
    };

    assert_eq!(expected, nm.to_resolved_tree());
    Ok(())
}

fn pkg(
    name: &str,
    version: &str,
    is_root: bool,
    path: Vec<UniCase<String>>,
) -> Result<PackageNode> {
    Ok(match (name, version) {
        ("a", "1") => PackageNode {
            name: "a".into(),
            is_root,
            path,
            version: Some("1.0.0".parse()?),
            resolved: None,
            integrity: None,
            dependencies: hashmap! {
                UniCase::new("b".to_owned()) => "b@^2".parse()?,
            },
            ..Default::default()
        },
        ("b", "2") => PackageNode {
            name: "b".into(),
            is_root,
            path,
            version: Some("2.0.0".parse()?),
            resolved: Some(PackageResolution::Npm {
                name: "b".into(),
                version: "2.0.0".parse()?,
                tarball: "https://example.com/b-2.0.0.tgz"
                    .parse()
                    .into_diagnostic()?,
                integrity: Some("sha512-badc0ffee".parse().into_diagnostic()?),
            }),
            dependencies: hashmap! {
                UniCase::new("c".to_owned()) => "c@^3".parse()?,
            },
            integrity: Some("sha512-badc0ffee".parse().into_diagnostic()?),
            ..Default::default()
        },
        ("c", "3") => PackageNode {
            name: "c".into(),
            is_root,
            path,
            version: Some("3.0.0".parse()?),
            resolved: Some(PackageResolution::Npm {
                name: "c".into(),
                version: "3.0.0".parse()?,
                tarball: "https://example.com/c-3.0.0.tgz"
                    .parse()
                    .into_diagnostic()?,
                integrity: Some("sha512-bad1dea".parse().into_diagnostic()?),
            }),
            integrity: Some("sha512-bad1dea".parse().into_diagnostic()?),
            dependencies: hashmap! {
                UniCase::new("d".to_owned()) => "d@^4".parse()?,
            },
            ..Default::default()
        },
        ("c", "5") => PackageNode {
            name: "c".into(),
            is_root,
            path,
            version: Some("5.0.0".parse()?),
            resolved: Some(PackageResolution::Npm {
                name: "c".into(),
                version: "5.0.0".parse()?,
                tarball: "https://example.com/c-5.0.0.tgz"
                    .parse()
                    .into_diagnostic()?,
                integrity: Some("sha512-12345".parse().into_diagnostic()?),
            }),
            integrity: Some("sha512-12345".parse().into_diagnostic()?),
            ..Default::default()
        },
        ("d", "4") => PackageNode {
            name: "d".into(),
            is_root,
            path,
            version: Some("4.0.0".parse()?),
            resolved: Some(PackageResolution::Npm {
                name: "d".into(),
                version: "4.0.0".parse()?,
                tarball: "https://example.com/d-4.0.0.tgz"
                    .parse()
                    .into_diagnostic()?,
                integrity: Some("sha512-54321".parse().into_diagnostic()?),
            }),
            integrity: Some("sha512-54321".parse().into_diagnostic()?),
            dependencies: hashmap! {
                UniCase::new("c".to_owned()) => "c@^5".parse()?,
            },
            ..Default::default()
        },
        (_, _) => panic!("unexpected package"),
    })
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
                        "b": "^2.0.0"
                    },
                    "dist": {
                        "integrity": "sha512-deadbeef",
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
                        "c": "^3.0.0"
                    },
                    "dist": {
                        "integrity": "sha512-badc0ffee",
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
                    "dependencies": {
                        "d": "^4.0.0"
                    },
                    "dist": {
                        "integrity": "sha512-bad1dea",
                        "tarball": "https://example.com/c-3.0.0.tgz"
                    }
                },
                "5.0.0": {
                    "name": "c",
                    "version": "5.0.0",
                    "dist": {
                        "integrity": "sha512-12345",
                        "tarball": "https://example.com/c-5.0.0.tgz"
                    }
                }
            },
            "dist-tags": {
                "latest": "5.0.0"
            }
        })))
        .mount(mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("d"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "versions": {
                "4.0.0": {
                    "name": "d",
                    "version": "4.0.0",
                    "dependencies": {
                        "c": "^5.0.0"
                    },
                    "dist": {
                        "integrity": "sha512-54321",
                        "tarball": "https://example.com/d-4.0.0.tgz"
                    }
                }
            },
            "dist-tags": {
                "latest": "4.0.0"
            }
        })))
        .mount(mock_server)
        .await;
}
