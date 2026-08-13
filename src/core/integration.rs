#![allow(dead_code)]
use crate::core::timeline::{Composition, Layer, LayerType};
use serde::{Deserialize, Serialize};

// --- 1. OpenTimelineIO (OTIO) / MLT Interoperability ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioItem {
    pub name: String,
    pub source_range: [u32; 2], // [start_frame, duration]
    pub media_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioTrack {
    pub name: String,
    pub kind: String, // "video" or "audio"
    pub items: Vec<OtioItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioTimeline {
    pub name: String,
    pub global_start_time: u32,
    pub fps: f64,
    pub tracks: Vec<OtioTrack>,
}

impl OtioTimeline {
    /// Convert our Composition model into an OpenTimelineIO-compatible data structure
    pub fn from_composition(comp: &Composition) -> Self {
        let mut video_track = OtioTrack {
            name: "Video Track 1".to_string(),
            kind: "video".to_string(),
            items: Vec::new(),
        };

        for layer in &comp.layers {
            let media_ref = match &layer.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Solid { .. } => Some("color_solid".to_string()),
                LayerType::Text { text, .. } => Some(format!("text:{}", text)),
                LayerType::Shape { .. } => Some("vector_shape".to_string()),
                LayerType::Null => None,
                LayerType::PreComp { comp_id } => Some(format!("precomp:{}", comp_id)),
                LayerType::Audio { path, .. } => Some(format!("audio:{}", path)),
            };

            video_track.items.push(OtioItem {
                name: layer.name.clone(),
                source_range: [layer.in_frame, layer.out_frame - layer.in_frame],
                media_reference: media_ref,
            });
        }

        Self {
            name: comp.name.clone(),
            global_start_time: 0,
            fps: comp.fps as f64,
            tracks: vec![video_track],
        }
    }

    /// Import and create a Composition model from an OTIO Timeline structure
    pub fn to_composition(&self) -> Composition {
        let mut comp = Composition::new(
            self.name.clone(),
            self.name.clone(),
            1920, // default width
            1080, // default height
            self.fps as u32,
            300, // default duration
        );

        let mut layer_idx = 0;
        for track in &self.tracks {
            if track.kind == "video" {
                for item in &track.items {
                    let layer_type = match &item.media_reference {
                        Some(ref_str) if ref_str.starts_with("text:") => LayerType::Text {
                            text: ref_str["text:".len()..].to_string(),
                            font_size: 48,
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        Some(ref_str) if ref_str == "color_solid" => LayerType::Solid {
                            color: [0.2, 0.5, 0.8, 1.0],
                        },
                        Some(ref_str) if ref_str == "vector_shape" => LayerType::Shape {
                            shape_type: crate::core::timeline::ShapeType::Rectangle,
                            color: [0.3, 0.6, 1.0, 1.0],
                        },
                        Some(path) => LayerType::Image { path: path.clone() },
                        None => {
                            LayerType::Solid {
                                color: [1.0, 0.0, 1.0, 1.0],
                            } // Placeholder Pink Solid
                        }
                    };

                    let mut layer = Layer::new(
                        format!("otio_layer_{}", layer_idx),
                        item.name.clone(),
                        layer_type,
                        item.source_range[1],
                    );
                    layer.in_frame = item.source_range[0];
                    layer.out_frame = item.source_range[0] + item.source_range[1];
                    comp.add_layer(layer);
                    layer_idx += 1;
                }
            }
        }

        comp
    }

    /// Serialize to JSON string for sending to Kdenlive / Shotcut (OTIO import)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string (OTIO export from Kdenlive / Shotcut)
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

// --- 2. Dynamic Link Server (Real-time synchronization protocol) ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DynamicLinkMessage {
    /// Synchronize playhead position
    SyncPlayhead {
        frame: u32,
    },
    /// Notify or request asset addition
    AddAsset {
        name: String,
        path: String,
    },
    /// Trigger frame render request from external NLE
    TriggerRender {
        comp_id: String,
        frame: u32,
    },
    /// Connection handshake / status
    Handshake {
        client_name: String,
        protocol_version: String,
    },
    Ping,
    Pong,
}

pub struct DynamicLinkServer {
    pub is_connected: bool,
    pub connected_app: Option<String>, // e.g. "Kdenlive" or "Shotcut"
    pub sync_port: u16,
}

impl Default for DynamicLinkServer {
    fn default() -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: 8080,
        }
    }
}

impl DynamicLinkServer {
    pub fn new(port: u16) -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: port,
        }
    }

    /// Process an incoming message from the WebSocket link and return a response message if necessary
    pub fn handle_message(&mut self, raw_msg: &str) -> Option<DynamicLinkMessage> {
        let msg: Result<DynamicLinkMessage, serde_json::Error> = serde_json::from_str(raw_msg);

        match msg {
            Ok(DynamicLinkMessage::Handshake { client_name, .. }) => {
                self.is_connected = true;
                self.connected_app = Some(client_name.clone());
                Some(DynamicLinkMessage::Handshake {
                    client_name: "AfterEffects-OSS-Alternative".to_string(),
                    protocol_version: "1.0.0".to_string(),
                })
            }
            Ok(DynamicLinkMessage::Ping) => Some(DynamicLinkMessage::Pong),
            Ok(DynamicLinkMessage::SyncPlayhead { .. }) => {
                // Return None, application layer will handle playhead update
                None
            }
            _ => None,
        }
    }
}

/// Spawns a background thread running a TCP synchronization server.
pub fn start_sync_server(
    port: u16,
    frame_tx: std::sync::mpsc::Sender<u32>,
    connection_tx: std::sync::mpsc::Sender<Option<String>>,
) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    thread::spawn(move || {
        let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind TCP sync port {}: {}", port, e);
                return;
            }
        };
        log::info!("Dynamic Link Sync Server listening on port {}", port);

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let stream_clone = match stream.try_clone() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
            let mut reader = BufReader::new(stream_clone);
            let mut writer = stream;

            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        // EOF - Connection closed
                        break;
                    }
                    Ok(_) => {
                        if line.len() > 65_536 {
                            log::warn!("TCP line limit exceeded ({} bytes). Closing connection to prevent memory exhaustion.", line.len());
                            break;
                        }
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Try to parse DynamicLinkMessage
                        if let Ok(msg) = serde_json::from_str::<DynamicLinkMessage>(trimmed) {
                            match msg {
                                DynamicLinkMessage::Handshake { client_name, .. } => {
                                    connection_tx.send(Some(client_name.clone())).ok();

                                    let resp = DynamicLinkMessage::Handshake {
                                        client_name: "AfterEffects-OSS-Alternative".to_string(),
                                        protocol_version: "1.0.0".to_string(),
                                    };
                                    if let Ok(resp_json) = serde_json::to_string(&resp) {
                                        writer.write_all(resp_json.as_bytes()).ok();
                                        writer.write_all(b"\n").ok();
                                        writer.flush().ok();
                                    }
                                }
                                DynamicLinkMessage::Ping => {
                                    let resp = DynamicLinkMessage::Pong;
                                    if let Ok(resp_json) = serde_json::to_string(&resp) {
                                        writer.write_all(resp_json.as_bytes()).ok();
                                        writer.write_all(b"\n").ok();
                                        writer.flush().ok();
                                    }
                                }
                                DynamicLinkMessage::SyncPlayhead { frame } => {
                                    frame_tx.send(frame).ok();
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_) => {
                        // Connection lost or read error
                        break;
                    }
                }
            }

            // Connection lost
            connection_tx.send(None).ok();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otio_serialization() {
        let mut comp = Composition::new(
            "test_comp".to_string(),
            "test_comp".to_string(),
            1920,
            1080,
            30,
            120,
        );
        let layer = Layer::new(
            "layer_1".to_string(),
            "Solid Layer".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            120,
        );
        comp.add_layer(layer);

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(otio.name, "test_comp");
        assert_eq!(otio.tracks[0].items[0].name, "Solid Layer");

        let json = otio.to_json().unwrap();
        let parsed_otio = OtioTimeline::from_json(&json).unwrap();
        assert_eq!(parsed_otio.name, "test_comp");

        let parsed_comp = parsed_otio.to_composition();
        assert_eq!(parsed_comp.layers.len(), 1);
        assert_eq!(parsed_comp.layers[0].name, "Solid Layer");
    }

    #[test]
    fn test_dynamic_link_handshake() {
        let mut server = DynamicLinkServer::new(9000);
        assert!(!server.is_connected);

        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        let resp = server.handle_message(handshake_req);

        assert!(server.is_connected);
        assert_eq!(server.connected_app.unwrap(), "Kdenlive");

        if let Some(DynamicLinkMessage::Handshake { client_name, .. }) = resp {
            assert_eq!(client_name, "AfterEffects-OSS-Alternative");
        } else {
            panic!("Expected handshake response");
        }
    }

    #[test]
    fn test_real_tcp_server_sync() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpStream;
        use std::sync::mpsc::channel;

        let (frame_tx, frame_rx) = channel();
        let (conn_tx, conn_rx) = channel();

        // Start server on a test port (e.g. 19001)
        start_sync_server(19001, frame_tx, conn_tx);

        // Connect client with retry loop
        let mut stream = None;
        for _ in 0..10 {
            if let Ok(s) = TcpStream::connect("127.0.0.1:19001") {
                stream = Some(s);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let mut stream = stream.expect("Failed to connect to test TCP server");
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        // Send Handshake Request
        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        stream.write_all(handshake_req.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Read Handshake Response
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("AfterEffects-OSS-Alternative"));

        // Verify connection state channel
        let conn_state = conn_rx.recv().unwrap();
        assert_eq!(conn_state, Some("Kdenlive".to_string()));

        // Send Playhead Sync
        let sync_msg = r#"{"type":"sync_playhead","frame":123}"#;
        stream.write_all(sync_msg.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Verify frame sync channel
        let frame = frame_rx.recv().unwrap();
        assert_eq!(frame, 123);
    }
}
