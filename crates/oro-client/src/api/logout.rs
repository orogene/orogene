use crate::{OroClient, OroClientError};

impl OroClient {
    pub async fn delete_token(&self, token: &String) -> Result<(), OroClientError> {
        self.client
            .delete(self.registry.join(&format!("-/user/token/{token}"))?)
            .header("X-Oro-Registry", self.registry.to_string())
            .send()
            .await?;
        Ok(())
    }
}
