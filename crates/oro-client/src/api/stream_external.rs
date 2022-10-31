use futures::{
    stream::{StreamExt, TryStreamExt},
    AsyncRead,
};
use url::Url;

use crate::{OroClient, OroClientError};

impl OroClient {
    pub async fn stream_external(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>, OroClientError> {
        Ok(Box::new(
            self.client
                .get(url.to_string())
                .send()
                .await?
                .bytes_stream()
                .map(|r| match r {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Error reading bytes",
                    )),
                })
                .into_async_read(),
        ))
    }
}
