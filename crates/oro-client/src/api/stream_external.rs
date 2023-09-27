use futures::{
    stream::{StreamExt, TryStreamExt},
    AsyncRead,
};
use url::Url;

use crate::{OroClient, OroClientError};

#[cfg(not(target_arch = "wasm32"))]
type Result = std::result::Result<Box<dyn AsyncRead + Unpin + Send + Sync>, OroClientError>;
#[cfg(target_arch = "wasm32")]
type Result = std::result::Result<Box<dyn AsyncRead + Unpin>, OroClientError>;

impl OroClient {
    pub async fn stream_external(&self, url: &Url) -> Result {
        Ok(Box::new(
            // NOTE: We don't want to cache these requests. If you want to
            // cache them, cache them manually.
            self.client_uncached
                .get(url.to_string())
                .header("X-Oro-Registry", self.registry.to_string())
                .send()
                .await?
                .error_for_status()?
                .bytes_stream()
                .map(|r| match r {
                    Ok(bytes) => Ok(bytes),
                    Err(err) => Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
                })
                .into_async_read(),
        ))
    }
}

#[cfg(test)]
mod test {
    use futures::AsyncReadExt;
    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[async_std::test]
    async fn stream_external() -> Result<()> {
        let mock_server = MockServer::start().await;
        let client: OroClient = Default::default();
        let server_url: Url = mock_server.uri().parse().into_diagnostic()?;

        {
            let _guard = Mock::given(method("GET"))
                .and(path("some/external/server"))
                .and(query_param("var", "val"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_raw("foo".as_bytes().to_owned(), "application/octet-stream"),
                )
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            let mut reader = client
                .stream_external(
                    &server_url
                        .join("some/external/server?var=val")
                        .into_diagnostic()?,
                )
                .await?;

            let mut data = Vec::new();
            reader.read_to_end(&mut data).await.into_diagnostic()?;

            assert_eq!(data, "foo".as_bytes().to_owned())
        }

        {
            let _guard = Mock::given(method("GET"))
                .and(path("some/external/server"))
                .and(query_param("var", "val"))
                .respond_with(ResponseTemplate::new(500))
                .expect(1)
                .mount_as_scoped(&mock_server)
                .await;

            assert!(matches!(
                client
                    .stream_external(
                        &server_url
                            .join("some/external/server?var=val")
                            .into_diagnostic()?
                    )
                    .await,
                Err(OroClientError::RequestError(_))
            ));
        }

        Ok(())
    }
}
