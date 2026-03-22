use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoclipError {
    #[error("No ASSEMBLYAI_API_KEY found. Set it as an environment variable or in a .env file.")]
    MissingApiKey,

    #[error("Failed to fetch temporary token: {0}")]
    TokenFetch(String),

    #[error("Audio device error: {0}")]
    AudioDevice(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Audio playback error: {0}")]
    Playback(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Keyboard input error: {0}")]
    Keyboard(String),

    #[error("Wake word error: {0}")]
    WakeWord(String),

    #[error("Invalid speech model: {0}. Use --list-models to see available models.")]
    InvalidModel(String),

    #[allow(dead_code)]
    #[error("API error: {0}")]
    Api(String),
}

impl From<reqwest::Error> for VoclipError {
    fn from(e: reqwest::Error) -> Self {
        VoclipError::TokenFetch(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for VoclipError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        VoclipError::WebSocket(e.to_string())
    }
}

impl From<arboard::Error> for VoclipError {
    fn from(e: arboard::Error) -> Self {
        VoclipError::Clipboard(e.to_string())
    }
}
