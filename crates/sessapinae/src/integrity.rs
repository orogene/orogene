use std::pin::Pin;
use std::task::{Context, Poll};

use oro_common::futures::{self, prelude::*};
use ssri::IntegrityOpts;

pub struct AsyncIntegrity<R: AsyncBufRead> {
    pub opts: IntegrityOpts,
    pub reader: R,
}

/*
impl<R: AsyncBufRead + Unpin> AsyncIntegrity<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            opts: IntegrityOpts::new().algorithm(Algorithm::Sha256),
        }
    }

    // pub fn into_inner(self) -> R {
    //     self.reader
    // }

    pub fn inner_result(self) -> (ssri::Integrity, R) {
        (self.opts.result(), self.reader)
    }
}
*/

impl<R: AsyncBufRead + Unpin> AsyncRead for AsyncIntegrity<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let amt = futures::ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
        self.opts.input(&buf[..amt]);
        Poll::Ready(Ok(amt))
    }
}
