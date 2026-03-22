use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::error::VoclipError;
use crate::resample::Resampler;

#[derive(Deserialize, Debug)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    transcript: Option<String>,
    end_of_turn: Option<bool>,
    error: Option<String>,
}

pub struct WsResult {
    pub transcript: String,
}

use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;

type WsTx = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsRx = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn connect(
    token: &str,
    timeout_secs: u32,
    speech_model: &str,
) -> Result<(WsTx, WsRx), VoclipError> {
    let timeout_ms = timeout_secs as u64 * 1000;

    let url = format!(
        "wss://streaming.assemblyai.com/v3/ws?token={token}\
         &speech_model={speech_model}\
         &sample_rate=16000\
         &encoding=pcm_s16le\
         &format_turns=true\
         &min_turn_silence={timeout_ms}\
         &max_turn_silence={timeout_ms}"
    );

    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
    Ok(ws_stream.split())
}

pub async fn stream(
    ws_tx: WsTx,
    ws_rx: WsRx,
    sample_rate: u32,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
) -> Result<WsResult, VoclipError> {
    let mut ws_tx = ws_tx;
    let mut ws_rx = ws_rx;

    let mut resampler = Resampler::new(sample_rate, 16000);

    // Shared transcript accumulator: receiver writes, main reads on completion
    let (done_tx, mut done_rx) = mpsc::channel::<String>(1);

    // Sender task: buffer audio to >= 100ms (1600 samples at 16kHz) per AssemblyAI requirement
    const MIN_SAMPLES: usize = 1600;
    let sender = tokio::spawn(async move {
        let mut buffer: Vec<i16> = Vec::with_capacity(MIN_SAMPLES * 2);
        while let Some(samples) = audio_rx.recv().await {
            let resampled = resampler.process(&samples);
            buffer.extend_from_slice(&resampled);

            if buffer.len() >= MIN_SAMPLES {
                let bytes: Vec<u8> = buffer.iter().flat_map(|s| s.to_le_bytes()).collect();
                buffer.clear();
                if ws_tx.send(Message::Binary(bytes)).await.is_err() {
                    break;
                }
            }
        }
        if !buffer.is_empty() {
            let bytes: Vec<u8> = buffer.iter().flat_map(|s| s.to_le_bytes()).collect();
            let _ = ws_tx.send(Message::Binary(bytes)).await;
        }
        let _ = ws_tx.close().await;
    });

    // Receiver task: parse WS messages, display partials, capture final transcript
    let receiver = tokio::spawn(async move {
        let mut full_transcript = String::new();

        while let Some(msg) = ws_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("\rWebSocket error: {e}");
                    break;
                }
            };

            let text = match &msg {
                Message::Text(t) => t.clone(),
                Message::Close(frame) => {
                    if let Some(f) = frame {
                        eprintln!("\rConnection closed by server: {} {}", f.code, f.reason);
                    }
                    break;
                }
                _ => continue,
            };

            let parsed: WsMessage = match serde_json::from_str(&text) {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Some(ref error) = parsed.error {
                eprintln!("\rAPI error: {error}");
                break;
            }

            match parsed.msg_type.as_deref().unwrap_or("") {
                "Begin" => {
                    eprintln!("Session started.");
                }
                "Turn" => {
                    let transcript = parsed.transcript.as_deref().unwrap_or("");
                    if parsed.end_of_turn == Some(true) {
                        if !transcript.is_empty() {
                            if !full_transcript.is_empty() {
                                full_transcript.push(' ');
                            }
                            full_transcript.push_str(transcript);
                        }
                        // Clear partial line, print final
                        eprint!("\r\x1b[2K");
                        if !transcript.is_empty() {
                            eprintln!("{transcript}");
                        }
                        let _ = done_tx.send(full_transcript).await;
                        return;
                    } else if !transcript.is_empty() {
                        eprint!("\r\x1b[2K{transcript}");
                    }
                }
                "Termination" => {
                    eprintln!("\rSession terminated by server.");
                    break;
                }
                other => {
                    eprintln!("\r[ws] unhandled message type: {other}");
                }
            }
        }

        // If we exit the loop without an end_of_turn, send what we have
        let _ = done_tx.send(full_transcript).await;
    });

    // Wait for final transcript or Ctrl+C
    let transcript = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprint!("\r\x1b[2K");
            eprintln!("Interrupted.");
            // Give receiver a moment to flush
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            done_rx.try_recv().unwrap_or_default()
        }
        result = done_rx.recv() => {
            result.unwrap_or_default()
        }
    };

    sender.abort();
    receiver.abort();

    Ok(WsResult { transcript })
}
