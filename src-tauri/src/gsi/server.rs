//! The GSI HTTP server: one `tiny_http` instance on its own thread.
//!
//! Flow per request: read body -> JSON parse -> token check -> merge into
//! `GsiShared` -> invoke the update callback (wired to a Tauri event in
//! lib.rs; a plain channel in tests). Malformed input never kills the loop.

use std::time::Instant;

use serde::Serialize;
use tiny_http::{Response, Server};

use super::payload::{GameState, GsiPayload};
use super::SharedGsi;

/// What the frontend's debug panel receives per accepted payload.
#[derive(Debug, Clone, Serialize)]
pub struct GsiUpdate {
    /// Raw payload as posted by CS2, with the `auth` block stripped.
    pub raw: serde_json::Value,
    /// Merged state after applying this payload.
    pub state: GameState,
}

/// Bind `addr` (use port 0 for an OS-assigned port in tests) and serve on a
/// detached thread until process exit. Returns the bound port.
pub fn start_server(
    addr: &str,
    token: String,
    shared: SharedGsi,
    on_update: impl Fn(GsiUpdate) + Send + 'static,
) -> Result<u16, String> {
    let server = Server::http(addr).map_err(|e| format!("bind {addr}: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("non-IP listen address")?;

    std::thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_err() {
                let _ = request.respond(Response::empty(400));
                continue;
            }
            let raw: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    let _ = request.respond(Response::empty(400));
                    continue;
                }
            };
            // Lenient typed view over the same JSON (unknown fields ignored).
            let payload: GsiPayload = match serde_json::from_value(raw.clone()) {
                Ok(p) => p,
                Err(_) => {
                    let _ = request.respond(Response::empty(400));
                    continue;
                }
            };
            let authorized = payload
                .auth
                .as_ref()
                .and_then(|a| a.token.as_deref())
                .map(|t| t == token)
                .unwrap_or(false);
            if !authorized {
                let _ = request.respond(Response::empty(401));
                continue;
            }

            let state = {
                let mut g = shared.lock().unwrap();
                g.state.apply(&payload);
                g.last_payload = Some(Instant::now());
                g.state.clone()
            };
            let mut redacted = raw;
            if let Some(obj) = redacted.as_object_mut() {
                obj.remove("auth");
            }
            on_update(GsiUpdate {
                raw: redacted,
                state,
            });
            let _ = request.respond(Response::empty(200));
        }
    });

    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc;

    /// Minimal raw HTTP POST; returns the status line (e.g. "HTTP/1.1 200 OK").
    fn post(port: u16, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            stream,
            "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response.lines().next().unwrap_or_default().to_string()
    }

    fn start_test_server() -> (u16, crate::gsi::SharedGsi, mpsc::Receiver<GsiUpdate>) {
        let shared: crate::gsi::SharedGsi = Default::default();
        let (tx, rx) = mpsc::channel();
        let port = start_server(
            "127.0.0.1:0", // OS-assigned free port
            "testtoken".to_string(),
            shared.clone(),
            move |u| {
                let _ = tx.send(u);
            },
        )
        .unwrap();
        (port, shared, rx)
    }

    #[test]
    fn accepts_valid_token_and_updates_state() {
        let (port, shared, rx) = start_test_server();
        let status = post(port, crate::gsi::payload::tests::SAMPLE_PAYLOAD);
        assert!(status.contains("200"), "got: {status}");

        let update = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(update.state.health, Some(87));
        // Token must not leak to the frontend.
        assert!(update.raw.get("auth").is_none());

        let g = shared.lock().unwrap();
        assert_eq!(g.state.round_phase.as_deref(), Some("live"));
        assert!(g.last_payload.is_some());
    }

    #[test]
    fn rejects_bad_token() {
        let (port, shared, rx) = start_test_server();
        let status = post(
            port,
            r#"{ "auth": { "token": "WRONG" }, "player": { "state": { "health": 1 } } }"#,
        );
        assert!(status.contains("401"), "got: {status}");
        assert!(rx.try_recv().is_err(), "callback must not fire");
        assert_eq!(shared.lock().unwrap().state.health, None);
    }

    #[test]
    fn rejects_missing_token_and_garbage_body() {
        let (port, _shared, rx) = start_test_server();
        assert!(post(port, r#"{ "player": {} }"#).contains("401"));
        assert!(post(port, "not json at all").contains("400"));
        assert!(rx.try_recv().is_err());
    }
}
