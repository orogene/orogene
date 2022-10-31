use crate::{OroClient, OroClientError};

impl OroClient {
    pub async fn ping(&self) -> Result<String, OroClientError> {
        Ok(self.client.get(self.registry.join("-/ping?write=true")?).send().await?.text().await?)
    }
}
