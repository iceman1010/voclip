use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::error::VoclipError;
use crate::speech_model::SpeechModel;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub default_timeout: Option<u32>,
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
}

impl Config {
    pub fn load(args: &Args) -> Result<Self, VoclipError> {
        let _ = dotenvy::dotenv();

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

        Ok(Config {
            api_key,
            timeout,
            model,
            delay: args.delay,
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

pub fn print_models() {
    println!("Available speech models:\n");
    for model in SpeechModel::all() {
        println!("  {:<18} {}", model.cli_name(), model.description());
    }
    println!();
    println!("Use --model <name> to select for one run.");
    println!("Use --set-default-model <name> to save as default.");
}
