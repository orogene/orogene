use futures::channel::oneshot;

use crate::error::AlabasterError;

pub(crate) async fn init() -> Result<(), AlabasterError> {
    let (send, recv) = oneshot::channel::<Option<AlabasterError>>();
    ffi::installFileProvider(Box::new(|err| {
        send.send(err.map(|err| AlabasterError::FileProviderInitError(err.message)))
            .unwrap();
        }));
    if let Some(err) = recv.await.expect("This operation won't be cancelled.") {
        Err(err)
    } else {
        Ok(())
    }
}

#[swift_bridge::bridge]
pub mod ffi {
    #[swift_bridge(swift_repr = "struct")]
    struct FileProviderError {
        message: String,
    }

    extern "Swift" {
        type FileProviderExtension;

        fn installFileProvider(completionHandler: Box<dyn FnOnce(Option<FileProviderError>)>);
    }
}
