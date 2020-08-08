extern crate oro_semver;

#[test]
fn test_regressions() {
    use oro_semver::ReqParseError;
    use oro_semver::VersionReq;

    let versions = vec![
        (".*", VersionReq::any()),
        ("0.1.0.", VersionReq::parse("0.1.0").unwrap()),
        ("0.3.1.3", VersionReq::parse("0.3.13").unwrap()),
        ("0.2*", VersionReq::parse("0.2.*").unwrap()),
        // TODO: this actually parses as '*' now, not sure if that's OK
        // ("*.0", VersionReq::any()),
    ];

    for (version, requirement) in versions.into_iter() {
        let parsed = VersionReq::parse(version);
        let error = parsed.err().unwrap();

        assert_eq!(
            ReqParseError::DeprecatedVersionRequirement(requirement),
            error
        );
    }
}
