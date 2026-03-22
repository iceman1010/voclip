use std::fmt;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::error::VoclipError;
use crate::speech_model::SpeechModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Clipboard,
    Type,
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputMode::Clipboard => write!(f, "clipboard"),
            OutputMode::Type => write!(f, "type"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "voclip", about = "Voice to clipboard — speak and paste")]
pub struct Args {
    /// Print version and exit
    #[arg(long)]
    pub version: bool,

    /// Check for updates and self-update if a newer version is available
    #[arg(long)]
    pub update: bool,

    /// Silence timeout in seconds
    #[arg(long, default_value_t = 3)]
    pub timeout: u32,

    /// Speech model to use (u3-rt-pro, english, multilingual, whisper-rt)
    #[arg(long)]
    pub model: Option<String>,

    /// Delay in seconds before starting to record
    #[arg(long, default_value_t = 1)]
    pub delay: u32,

    /// List available speech models and exit
    #[arg(long)]
    pub list_models: bool,

    /// Set the default speech model and save to config
    #[arg(long)]
    pub set_default_model: Option<String>,

    /// Set the default silence timeout (seconds) and save to config
    #[arg(long)]
    pub set_default_timeout: Option<u32>,

    /// Type text via keyboard instead of copying to clipboard
    #[arg(long)]
    pub r#type: bool,

    /// Set the default output mode and save to config (clipboard or type)
    #[arg(long)]
    pub set_default_output: Option<String>,

    /// Run in always-on listen mode with wake word detection (output is always typed)
    #[arg(long)]
    pub listen: bool,

    /// Record wake word samples and build a .rpw reference file
    #[arg(long)]
    pub train_wakeword: bool,

    /// Test/debug wake word detection — listen and print detection scores
    #[arg(long)]
    pub test_wakeword: bool,

    /// Label for the wake word — cosmetic only, shown in detection logs (used with --train-wakeword)
    #[arg(long, default_value = "hey voclip", requires = "train_wakeword")]
    pub wakeword_name: String,

    /// Number of samples to record during training (used with --train-wakeword)
    #[arg(long, default_value_t = 8, requires = "train_wakeword")]
    pub wakeword_samples: u32,

    /// Wake word detection sensitivity: low, medium, high (default: medium)
    #[arg(long, default_value = "medium")]
    pub wakeword_sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub default_timeout: Option<u32>,
    #[serde(default)]
    pub default_output: Option<String>,
    #[serde(default)]
    pub wakeword_path: Option<String>,
    #[serde(default)]
    pub wakeword_sensitivity: Option<String>,
}

impl ConfigFile {
    fn path() -> std::path::PathBuf {
        let config_dir = dirs_next::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        config_dir.join("voclip").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return ConfigFile::default();
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), VoclipError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| VoclipError::Config(e.to_string()))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| VoclipError::Config(e.to_string()))?;
        std::fs::write(&path, content).map_err(|e| VoclipError::Config(e.to_string()))
    }
}

pub struct Config {
    pub api_key: String,
    pub timeout: u32,
    pub model: SpeechModel,
    pub delay: u32,
    pub output_mode: OutputMode,
    pub wakeword_path: std::path::PathBuf,
    pub wakeword_sensitivity: WakewordSensitivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakewordSensitivity {
    Low,
    Medium,
    High,
}

impl WakewordSensitivity {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

impl Config {
    pub fn load(args: &Args) -> Result<Self, VoclipError> {
        // Load .env from current directory (if present)
        let _ = dotenvy::dotenv();
        // Also load from ~/.config/voclip/.env (for hotkey/autostart use)
        if let Some(config_dir) = dirs_next::config_dir() {
            let _ = dotenvy::from_path(config_dir.join("voclip").join(".env"));
        }

        let api_key =
            std::env::var("ASSEMBLYAI_API_KEY").map_err(|_| VoclipError::MissingApiKey)?;

        let file_config = ConfigFile::load();

        let model = if let Some(ref name) = args.model {
            SpeechModel::from_name(name)
                .ok_or_else(|| VoclipError::InvalidModel(name.clone()))?
        } else {
            match file_config.default_model {
                Some(ref name) => SpeechModel::from_name(name).unwrap_or(SpeechModel::U3RtPro),
                None => SpeechModel::U3RtPro,
            }
        };

        let timeout = if args.timeout != 3 {
            args.timeout
        } else {
            file_config.default_timeout.unwrap_or(3)
        };

        let output_mode = if args.r#type {
            OutputMode::Type
        } else {
            match file_config.default_output.as_deref() {
                Some("type") => OutputMode::Type,
                _ => OutputMode::Clipboard,
            }
        };

        let wakeword_path = file_config
            .wakeword_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(default_wakeword_path);

        let wakeword_sensitivity = if args.wakeword_sensitivity != "medium" {
            WakewordSensitivity::from_name(&args.wakeword_sensitivity)
                .unwrap_or(WakewordSensitivity::Medium)
        } else {
            file_config
                .wakeword_sensitivity
                .as_deref()
                .and_then(WakewordSensitivity::from_name)
                .unwrap_or(WakewordSensitivity::Medium)
        };

        Ok(Config {
            api_key,
            timeout,
            model,
            delay: args.delay,
            output_mode,
            wakeword_path,
            wakeword_sensitivity,
        })
    }
}

pub fn save_default_model(name: &str) -> Result<SpeechModel, VoclipError> {
    let model = SpeechModel::from_name(name)
        .ok_or_else(|| VoclipError::InvalidModel(name.to_string()))?;
    let mut config = ConfigFile::load();
    config.default_model = Some(model.cli_name().to_string());
    config.save()?;
    Ok(model)
}

pub fn save_default_timeout(secs: u32) -> Result<(), VoclipError> {
    let mut config = ConfigFile::load();
    config.default_timeout = Some(secs);
    config.save()
}

pub fn save_default_output(mode: &str) -> Result<OutputMode, VoclipError> {
    let output_mode = match mode {
        "clipboard" => OutputMode::Clipboard,
        "type" => OutputMode::Type,
        _ => {
            return Err(VoclipError::Config(format!(
                "Invalid output mode: {mode}. Use 'clipboard' or 'type'."
            )));
        }
    };
    let mut config = ConfigFile::load();
    config.default_output = Some(mode.to_string());
    config.save()?;
    Ok(output_mode)
}

pub fn default_wakeword_path() -> std::path::PathBuf {
    let config_dir = dirs_next::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    config_dir.join("voclip").join("wakeword.rpw")
}

pub fn print_models() {
    println!("Available speech models:\n");
    for model in SpeechModel::all() {
        println!("  {:<18} {}", model.cli_name(), model.description());
    }
    println!();
    println!("Use --model <name> to select for one run.");
    println!("Use --set-default-model <name> to save as default.");
}
