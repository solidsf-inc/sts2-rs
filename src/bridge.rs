use crate::driver::SimulatorDriver;
use crate::protocol::Command;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Response, Server, StatusCode};

pub struct BridgeServer<D: SimulatorDriver + Send + 'static> {
    driver: Arc<Mutex<D>>,
    port: u16,
}

impl<D: SimulatorDriver + Send + 'static> BridgeServer<D> {
    pub fn new(driver: D, port: u16) -> Self {
        Self {
            driver: Arc::new(Mutex::new(driver)),
            port,
        }
    }

    pub fn run(&self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| anyhow::anyhow!("Failed to bind HTTP server to {}: {}", addr, e))?;

        println!(
            "⚡ STS2 Rust Bridge listening on http://localhost:{}",
            self.port
        );

        for mut request in server.incoming_requests() {
            let driver_clone = Arc::clone(&self.driver);

            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                let err_resp = format!("{{\"type\":\"error\",\"message\":\"{}\"}}", e);
                let _ = request
                    .respond(Response::from_string(err_resp).with_status_code(StatusCode(400)));
                continue;
            }

            let cmd_res: Result<Command, _> = serde_json::from_str(&body);
            match cmd_res {
                Ok(cmd) => {
                    let mut drv = driver_clone.lock().unwrap();
                    match drv.send(&cmd) {
                        Ok(state) => {
                            let resp_json = serde_json::to_string(&state).unwrap_or_default();
                            let response = Response::from_string(resp_json).with_header(
                                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                    .unwrap(),
                            );
                            let _ = request.respond(response);
                        }
                        Err(e) => {
                            let err_resp = format!("{{\"type\":\"error\",\"message\":\"{}\"}}", e);
                            let response = Response::from_string(err_resp)
                                .with_status_code(StatusCode(500))
                                .with_header(
                                    Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/json"[..],
                                    )
                                    .unwrap(),
                                );
                            let _ = request.respond(response);
                        }
                    }
                }
                Err(e) => {
                    let err_resp =
                        format!("{{\"type\":\"error\",\"message\":\"Invalid JSON: {}\"}}", e);
                    let response = Response::from_string(err_resp)
                        .with_status_code(StatusCode(400))
                        .with_header(
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap(),
                        );
                    let _ = request.respond(response);
                }
            }
        }

        Ok(())
    }
}
