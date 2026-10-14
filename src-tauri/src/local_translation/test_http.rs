//! Test-only loopback HTTP/1.1 capture server shared by the translation
//! client and worker tests. Each server accepts exactly one connection,
//! records the request, and answers with a canned status line + body. A
//! second request from the client therefore fails to connect, which lets
//! tests assert how many engine calls were made by whether the expected
//! number of snapshots arrived.

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub(crate) struct CapturedRequest {
    raw: Vec<u8>,
}

impl CapturedRequest {
    pub(crate) fn head(&self) -> String {
        let end = self
            .find_head_end()
            .expect("request must contain a header terminator");
        String::from_utf8_lossy(&self.raw[..end]).into_owned()
    }

    pub(crate) fn body(&self) -> &[u8] {
        let end = self
            .find_head_end()
            .expect("request must contain a header terminator");
        &self.raw[end..]
    }

    fn find_head_end(&self) -> Option<usize> {
        self.raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|pos| pos + 4)
    }
}

pub(crate) async fn spawn_capture_server(
    status_line: &'static str,
    body: &'static str,
) -> (SocketAddr, JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener local addr");
    let handle = tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut socket, _) = listener.accept().await.expect("client connection");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let read = socket.read(&mut chunk).await.expect("read request head");
            assert!(read > 0, "client closed before sending a request");
            request.extend_from_slice(&chunk[..read]);
            if let Some(pos) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let head = String::from_utf8_lossy(&request[..header_end]).to_string();
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = socket.read(&mut chunk).await.expect("read request body");
            assert!(read > 0, "client closed before sending the body");
            request.extend_from_slice(&chunk[..read]);
        }
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
        socket.shutdown().await.expect("shutdown socket");
        CapturedRequest { raw: request }
    });
    (addr, handle)
}
