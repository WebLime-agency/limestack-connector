use crate::printer;
use crate::protocol::{ClientMessage, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

const CONNECTOR_VERSION: &str = env!("CARGO_PKG_VERSION");
const SERVER_PORT: u16 = 9632;

/// Allowed origins for WebSocket connections
const ALLOWED_ORIGINS: &[&str] = &[
    "https://app.limestack.io",
    "https://limestack.io",
    "http://localhost:5173", // Local dev
    "http://localhost:4173", // Local preview
];

pub async fn start_server(_app_handle: tauri::AppHandle) {
    let addr = SocketAddr::from(([127, 0, 0, 1], SERVER_PORT));

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            log::info!("WebSocket server listening on ws://127.0.0.1:{}", SERVER_PORT);
            l
        }
        Err(e) => {
            log::error!("Failed to bind to port {}: {}", SERVER_PORT, e);
            return;
        }
    };

    while let Ok((stream, peer_addr)) = listener.accept().await {
        log::info!("New connection from: {}", peer_addr);
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut authenticated = false;

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => {
                log::info!("Client disconnected");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Invalid message: {}", e);
                let error = ServerMessage::Error {
                    message: format!("Invalid message format: {}", e),
                };
                let _ = write.send(Message::Text(serde_json::to_string(&error).unwrap())).await;
                continue;
            }
        };

        let response = match client_msg {
            ClientMessage::Hello { version: _, origin } => {
                // Validate origin
                if !ALLOWED_ORIGINS.iter().any(|o| origin.starts_with(o)) {
                    log::warn!("Rejected connection from origin: {}", origin);
                    ServerMessage::Error {
                        message: "Origin not allowed".to_string(),
                    }
                } else {
                    authenticated = true;
                    log::info!("Client authenticated from origin: {}", origin);
                    ServerMessage::Welcome {
                        connector_version: CONNECTOR_VERSION.to_string(),
                        capabilities: vec!["print".to_string()],
                        printers: printer::get_printers(),
                    }
                }
            }

            ClientMessage::GetPrinters => {
                if !authenticated {
                    ServerMessage::Error {
                        message: "Not authenticated".to_string(),
                    }
                } else {
                    ServerMessage::Printers {
                        printers: printer::get_printers(),
                    }
                }
            }

            ClientMessage::Print {
                request_id,
                printer: printer_id,
                format,
                data,
                options,
            } => {
                if !authenticated {
                    ServerMessage::Error {
                        message: "Not authenticated".to_string(),
                    }
                } else {
                    handle_print_request(request_id, printer_id, data, format, options.copies.unwrap_or(1))
                }
            }

            ClientMessage::ReadScale => {
                // TODO: Implement scale reading
                ServerMessage::Error {
                    message: "Scale reading not yet implemented".to_string(),
                }
            }
        };

        let response_json = serde_json::to_string(&response).unwrap();
        if let Err(e) = write.send(Message::Text(response_json)).await {
            log::error!("Failed to send response: {}", e);
            break;
        }
    }
}

fn handle_print_request(
    request_id: String,
    printer_id: String,
    data: String,
    format: String,
    copies: u32,
) -> ServerMessage {
    log::info!("Print request for printer: {} (format: {})", printer_id, format);

    // Find the printer
    let printer_name = match printer::find_printer(&printer_id) {
        Some(name) => name,
        None => {
            return ServerMessage::PrintResult {
                request_id,
                success: false,
                message: None,
                error: Some(format!("Printer not found: {}", printer_id)),
            };
        }
    };

    // Print the label
    match printer::print_label(&printer_name, &data, &format, copies) {
        Ok(_) => {
            log::info!("Print job sent successfully to {}", printer_name);
            ServerMessage::PrintResult {
                request_id,
                success: true,
                message: Some(format!("Label sent to {}", printer_name)),
                error: None,
            }
        }
        Err(e) => {
            log::error!("Print failed: {}", e);
            ServerMessage::PrintResult {
                request_id,
                success: false,
                message: None,
                error: Some(e),
            }
        }
    }
}
