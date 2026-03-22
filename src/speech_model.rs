use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechModel {
    U3RtPro,
    UniversalEnglish,
    UniversalMultilingual,
    WhisperRt,
}

impl SpeechModel {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" | "u3-rt-pro" | "u3" => Some(SpeechModel::U3RtPro),
            "english" | "en" => Some(SpeechModel::UniversalEnglish),
            "multilingual" | "multi" => Some(SpeechModel::UniversalMultilingual),
            "whisper" | "whisper-rt" => Some(SpeechModel::WhisperRt),
            _ => None,
        }
    }

    pub fn api_name(&self) -> &'static str {
        match self {
            SpeechModel::U3RtPro => "u3-rt-pro",
            SpeechModel::UniversalEnglish => "universal-streaming-english",
            SpeechModel::UniversalMultilingual => "universal-streaming-multilingual",
            SpeechModel::WhisperRt => "whisper-rt",
        }
    }

    pub fn cli_name(&self) -> &'static str {
        match self {
            SpeechModel::U3RtPro => "u3-rt-pro",
            SpeechModel::UniversalEnglish => "english",
            SpeechModel::UniversalMultilingual => "multilingual",
            SpeechModel::WhisperRt => "whisper-rt",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SpeechModel::U3RtPro => "Latest model, best quality (default)",
            SpeechModel::UniversalEnglish => "English only, optimized for English speech",
            SpeechModel::UniversalMultilingual => "Multi-language support, auto-detects language",
            SpeechModel::WhisperRt => "Whisper-based real-time model",
        }
    }

    pub fn all() -> &'static [SpeechModel] {
        &[
            SpeechModel::U3RtPro,
            SpeechModel::UniversalEnglish,
            SpeechModel::UniversalMultilingual,
            SpeechModel::WhisperRt,
        ]
    }
}

impl fmt::Display for SpeechModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.cli_name())
    }
}
