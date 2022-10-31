use oro_common::Packument;

use crate::{OroClient, OroClientError};

impl OroClient {
    pub async fn packument(
        &self,
        package_name: impl AsRef<str>,
        use_corgi: bool,
    ) -> Result<Packument, OroClientError> {
        let packument = self
            .client
            .get(self.registry.join(package_name.as_ref())?)
            .header(
                "Accept",
                if use_corgi {
                    "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"
                } else {
                    "application/json"
                },
            )
            .send()
            .await?
            .json()
            .await?;

        Ok(packument)
    }
}
