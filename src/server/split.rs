//use bytes::{Buf, BufMut};
//use std::io;
//use std::net::Shutdown;
//use std::pin::Pin;
//use std::task::{Context, Poll};
//
//use std::sync::Arc;
//use tokio::net::TcpStream;
//use tokio::io::{AsyncRead, AsyncWrite};
//use futures::AsyncRead;
//use std::ops::Deref;
//
//#[derive(Debug)]
//pub struct ReadHalf(Arc<TcpStream>);
//
//#[derive(Debug)]
//pub struct WriteHalf(Arc<TcpStream>);
//
//pub fn split(stream: TcpStream) -> (ReadHalf, WriteHalf) {
//    let stream = Arc::new(stream);
//    let stream2 = Arc::clone(&stream);
//    (ReadHalf(stream), WriteHalf(stream2))
//}
//
//impl AsyncRead for ReadHalf {
//    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
//        false
//    }
//
//    fn poll_read(
//        self: Pin<&mut Self>,
//        cx: &mut Context<'_>,
//        buf: &mut [u8],
//    ) -> Poll<io::Result<usize>> {
//        self.0.poll_read(cx, buf)
//    }
//
//    fn poll_read_buf<B: BufMut>(
//        self: Pin<&mut Self>,
//        cx: &mut Context<'_>,
//        buf: &mut B,
//    ) -> Poll<io::Result<usize>> {
//        self.0.poll_read_buf(cx, buf)
//    }
//}
//
//impl AsyncWrite for WriteHalf {
//    fn poll_write(
//        self: Pin<&mut Self>,
//        cx: &mut Context<'_>,
//        buf: &[u8],
//    ) -> Poll<io::Result<usize>> {
//        self.0.poll_write(cx, buf)
//    }
//
//    #[inline]
//    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
//        Poll::Ready(Ok(()))
//    }
//
//    // `poll_shutdown` on a write half shutdowns the stream in the "write" direction.
//    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
//        self.0.shutdown(Shutdown::Write).into()
//    }
//
//    fn poll_write_buf<B: Buf>(
//        self: Pin<&mut Self>,
//        cx: &mut Context<'_>,
//        buf: &mut B,
//    ) -> Poll<io::Result<usize>> {
//        self.0.poll_write_buf(cx, buf)
//    }
//}
//
//impl AsRef<TcpStream> for ReadHalf {
//    fn as_ref(&self) -> &TcpStream {
//        self.0.as_ref()
//    }
//}
//
//impl AsRef<TcpStream> for WriteHalf {
//    fn as_ref(&self) -> &TcpStream {
//        self.0.as_ref()
//    }
//}
//
