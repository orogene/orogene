use std::fmt;

use oro_node_semver::VersionReq as Range;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitInfo {
    Hosted {
        owner: String,
        repo: String,
        host: String,
        committish: Option<String>,
        semver: Option<Range>,
        requested: Option<String>,
    },
    Url {
        url: Url,
        committish: Option<String>,
        semver: Option<Range>,
    },
    Ssh {
        ssh: String,
        committish: Option<String>,
        semver: Option<Range>,
    },
}

impl fmt::Display for GitInfo {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
