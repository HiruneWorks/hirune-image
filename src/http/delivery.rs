use crate::execution::{JobLease, PreparedResponse};
use crate::transport::Connection;
use application::error::{ProcessingError as Error, Result};
use axum::{
    body::Body,
    http::{Response, header},
};
use bytes::Bytes;
use http_body::{Body as HttpBody, Frame};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
struct DeliveryState {
    chunks: VecDeque<Bytes>,
    lease: Option<JobLease>,
    last_progress: Instant,
    timed_out: bool,
    waker: Option<Waker>,
}
struct DeliveryBody {
    state: Arc<Mutex<DeliveryState>>,
    finished: CancellationToken,
}
impl HttpBody for DeliveryBody {
    type Data = Bytes;
    type Error = std::io::Error;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<Frame<Bytes>, Self::Error>>> {
        let mut state = self.state.lock().expect("delivery mutex");
        if state.timed_out {
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "response delivery timed out",
            ))));
        }
        state.waker = Some(cx.waker().clone());
        if let Some(chunk) = state.chunks.pop_front() {
            state.last_progress = Instant::now();
            Poll::Ready(Some(Ok(Frame::data(chunk))))
        } else {
            state.lease.take();
            self.finished.cancel();
            Poll::Ready(None)
        }
    }
}
impl Drop for DeliveryBody {
    fn drop(&mut self) {
        let mut state = self.state.lock().expect("delivery mutex");
        state.chunks.clear();
        state.lease.take();
        self.finished.cancel();
    }
}
pub fn response(
    prepared: PreparedResponse,
    request_id: Uuid,
    deadline: Instant,
    idle_ms: u64,
    shutdown: CancellationToken,
    connection: Option<Connection>,
) -> Result<Response<Body>> {
    let boundary = format!("hirune-{request_id}");
    let mut chunks = VecDeque::new();
    let manifest = serde_json::to_vec(&prepared.manifest).map_err(|_| Error::InternalError)?;
    chunks.push_back(Bytes::from(format!("--{boundary}\r\nContent-Type: application/json\r\nContent-ID: <manifest>\r\nContent-Length: {}\r\n\r\n",manifest.len())));
    chunks.push_back(Bytes::from(manifest));
    chunks.push_back(Bytes::from_static(b"\r\n"));
    for (output, bytes) in prepared.manifest.outputs.iter().zip(prepared.outputs) {
        chunks.push_back(Bytes::from(format!("--{boundary}\r\nContent-Type: image/webp\r\nContent-ID: <output-{}>\r\nContent-Length: {}\r\n\r\n",output.id,bytes.len())));
        let mut offset = 0;
        while offset < bytes.len() {
            let end = (offset + 64 * 1024).min(bytes.len());
            chunks.push_back(Bytes::copy_from_slice(&bytes[offset..end]));
            offset = end;
        }
        chunks.push_back(Bytes::from_static(b"\r\n"));
    }
    chunks.push_back(Bytes::from(format!("--{boundary}--\r\n")));
    let length: usize = chunks.iter().map(Bytes::len).sum();
    let state = Arc::new(Mutex::new(DeliveryState {
        chunks,
        lease: Some(prepared.lease),
        last_progress: Instant::now(),
        timed_out: false,
        waker: None,
    }));
    let finished = CancellationToken::new();
    let body = DeliveryBody {
        state: state.clone(),
        finished: finished.clone(),
    };
    // This watchdog owns the lease even when a transport stops polling the body.
    tokio::spawn(async move {
        loop {
            let until = {
                let state = state.lock().expect("delivery mutex");
                deadline.min(state.last_progress + Duration::from_millis(idle_ms))
            };
            tokio::select! {_=finished.cancelled()=>return,_=shutdown.cancelled()=>{},_=tokio::time::sleep_until(until)=>{}}
            let mut state = state.lock().expect("delivery mutex");
            // Completion can win while the watchdog waits for the delivery lock.
            if finished.is_cancelled() {
                return;
            }
            if shutdown.is_cancelled()
                || Instant::now() >= deadline
                || Instant::now() >= state.last_progress + Duration::from_millis(idle_ms)
            {
                state.timed_out = true;
                if let Some(connection) = &connection {
                    connection.abort();
                }
                state.chunks.clear();
                state.lease.take();
                if let Some(waker) = state.waker.take() {
                    waker.wake()
                }
                return;
            }
        }
    });
    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/related; boundary={boundary}; type=\"application/json\""),
        )
        .header(header::CONTENT_LENGTH, length)
        .header(header::CACHE_CONTROL, "no-store")
        .header("x-request-id", request_id.to_string())
        .header("x-content-type-options", "nosniff")
        .body(Body::new(body))
        .map_err(|_| Error::InternalError)
}
