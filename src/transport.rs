use axum::{
    extract::connect_info::Connected,
    serve::{IncomingStream, Listener},
};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct Connection {
    inner: Arc<ConnectionInner>,
}
#[derive(Debug)]
struct ConnectionInner {
    socket: Mutex<Option<TcpStream>>,
    closed: CancellationToken,
    read_waker: futures_util::task::AtomicWaker,
    write_waker: futures_util::task::AtomicWaker,
}
impl Connection {
    fn new(socket: Option<TcpStream>) -> Self {
        Self {
            inner: Arc::new(ConnectionInner {
                socket: Mutex::new(socket),
                closed: CancellationToken::new(),
                read_waker: Default::default(),
                write_waker: Default::default(),
            }),
        }
    }
    pub fn close(&self) {
        self.close_socket(false);
    }
    pub fn abort(&self) {
        self.close_socket(true);
    }
    fn close_socket(&self, discard_pending_writes: bool) {
        if let Some(socket) = self.inner.socket.lock().expect("socket mutex").take() {
            // On abort, discard queued kernel writes. On normal close, let them drain.
            if discard_pending_writes
                && let Err(error) = socket2::SockRef::from(&socket).set_linger(Some(Duration::ZERO))
            {
                tracing::warn!(kind=?error.kind(), "TCP abort configuration failed");
            }
        }
        self.inner.closed.cancel();
        self.inner.read_waker.wake();
        self.inner.write_waker.wake();
    }
    pub fn closed(&self) -> CancellationToken {
        self.inner.closed.clone()
    }
}
#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub peer: SocketAddr,
    pub connection: Connection,
}
pub struct BoundedListener {
    pub listener: TcpListener,
    pub shutdown: CancellationToken,
}
pub struct GuardedIo {
    connection: Connection,
}
impl Drop for GuardedIo {
    fn drop(&mut self) {
        self.connection.close()
    }
}
impl AsyncRead for GuardedIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.connection.inner.read_waker.register(cx.waker());
        let mut socket = self.connection.inner.socket.lock().expect("socket mutex");
        let Some(socket) = socket.as_mut() else {
            return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
        };
        Pin::new(socket).poll_read(cx, buffer)
    }
}
impl AsyncWrite for GuardedIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.connection.inner.write_waker.register(cx.waker());
        let mut socket = self.connection.inner.socket.lock().expect("socket mutex");
        let Some(socket) = socket.as_mut() else {
            return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
        };
        Pin::new(socket).poll_write(cx, buffer)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.connection.inner.write_waker.register(cx.waker());
        let mut socket = self.connection.inner.socket.lock().expect("socket mutex");
        let Some(socket) = socket.as_mut() else {
            return Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into()));
        };
        Pin::new(socket).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut socket = self.connection.inner.socket.lock().expect("socket mutex");
        let Some(socket) = socket.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        Pin::new(socket).poll_shutdown(cx)
    }
}
impl Listener for BoundedListener {
    type Io = GuardedIo;
    type Addr = ConnectionInfo;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            match self.listener.accept().await {
                Ok((socket, peer)) => {
                    let connection = Connection::new(Some(socket));
                    let weak = Arc::downgrade(&connection.inner);
                    let closed = connection.closed();
                    let shutdown = self.shutdown.clone();
                    tokio::spawn(async move {
                        tokio::select! {_=closed.cancelled()=>{},_=shutdown.cancelled()=>{if let Some(inner)=weak.upgrade(){Connection{inner}.abort();}}}
                    });
                    return (
                        GuardedIo {
                            connection: connection.clone(),
                        },
                        ConnectionInfo { peer, connection },
                    );
                }
                Err(error) => {
                    tracing::warn!(kind=?error.kind(),"TCP accept failed");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(ConnectionInfo {
            peer: self.listener.local_addr()?,
            connection: Connection::new(None),
        })
    }
}
impl Connected<IncomingStream<'_, BoundedListener>> for ConnectionInfo {
    fn connect_info(stream: IncomingStream<'_, BoundedListener>) -> Self {
        stream.remote_addr().clone()
    }
}
