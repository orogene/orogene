pub(crate) fn init() {
    ffi::FileProviderExtension::install(Box::new(|err| {
        if let Some(err) = err {
            ffi::print(err);
            panic!("Got an err initializing")
        }
    }));
}

#[swift_bridge::bridge]
pub mod ffi {
    struct FileProviderError;

    extern "Swift" {
        fn print(x: FileProviderError);
    }

    extern "Swift" {
        type FileProviderExtension;

        #[swift_bridge(associated_to = FileProviderExtension)]
        fn install(completionHandler: Box<dyn FnOnce(Option<FileProviderError>)>);
    }
}
