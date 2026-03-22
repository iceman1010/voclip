use std::fmt;
use std::path::PathBuf;

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

    /// Run in always-on listen mode with wake word and command word detection
    #[arg(long)]
    pub listen: bool,

    /// Train the wake word (triggers transcription)
    #[arg(long)]
    pub train_wakeword: bool,

    /// Train a command word (triggers a key press action)
    #[arg(long)]
    pub train_command: bool,

    /// Test/debug detection of all trained voice patterns
    #[arg(long)]
    pub test_wakeword: bool,

    /// Name for the wake word (used with --train-wakeword)
    #[arg(long, default_value = "hey voclip")]
    pub wakeword_name: String,

    /// Name for the command word (used with --train-command)
    #[arg(long)]
    pub command_name: Option<String>,

    /// Action for the command word: "key:<keyname>" e.g. "key:Return" (used with --train-command)
    #[arg(long)]
    pub command_action: Option<String>,

    /// Number of samples to record during training
    #[arg(long, default_value_t = 8)]
    pub wakeword_samples: u32,

    /// Detection sensitivity: low, medium, high, or a number like 0.5 (default: medium)
    #[arg(long, default_value = "medium")]
    pub wakeword_sensitivity: String,

    /// List all configured wake word and command words
    #[arg(long)]
    pub list_wakewords: bool,

    /// Remove a trained voice pattern by name
    #[arg(long)]
    pub remove_wakeword: Option<String>,
}

// --- Voice pattern types ---

#[derive(Debug, Clone, PartialEq)]
pub enum VoiceAction {
    Transcribe,
    Key(String),
}

impl fmt::Display for VoiceAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoiceAction::Transcribe => write!(f, "transcribe"),
            VoiceAction::Key(key) => write!(f, "key:{key}"),
        }
    }
}

impl VoiceAction {
    pub fn parse(s: &str) -> Option<Self> {
        if s == "transcribe" {
            Some(VoiceAction::Transcribe)
        } else if let Some(key) = s.strip_prefix("key:") {
            if key.is_empty() {
                None
            } else {
                Some(VoiceAction::Key(key.to_string()))
            }
        } else {
            None
        }
    }

    pub fn to_config_string(&self) -> String {
        match self {
            VoiceAction::Transcribe => "transcribe".to_string(),
            VoiceAction::Key(key) => format!("key:{key}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoicePattern {
    pub name: String,
    pub path: PathBuf,
    pub action: VoiceAction,
}

// --- Config file (TOML) ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoicePatternEntry {
    pub name: String,
    pub action: String,
    #[serde(default)]
    pub path: Option<String>,
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
    pub wakeword_sensitivity: Option<String>,

    // Legacy single-wakeword fields (for backward compatibility)
    #[serde(default)]
    pub wakeword_path: Option<String>,
    #[serde(default)]
    pub wakeword_name: Option<String>,

    // Multi-pattern config
    #[serde(default)]
    pub voice_pattern: Vec<VoicePatternEntry>,
}

impl ConfigFile {
    fn path() -> PathBuf {
        let config_dir = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
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

// --- Sensitivity ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WakewordSensitivity {
    Low,
    Medium,
    High,
    Custom(f32),
}

impl WakewordSensitivity {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => value.parse::<f32>().ok().map(Self::Custom),
        }
    }
}

// --- Runtime config ---

pub struct Config {
    pub api_key: String,
    pub timeout: u32,
    pub model: SpeechModel,
    pub delay: u32,
    pub output_mode: OutputMode,
    pub wakeword_sensitivity: WakewordSensitivity,
    pub voice_patterns: Vec<VoicePattern>,
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

        let wakeword_sensitivity = if args.wakeword_sensitivity != "medium" {
            WakewordSensitivity::parse(&args.wakeword_sensitivity)
                .unwrap_or(WakewordSensitivity::Medium)
        } else {
            file_config
                .wakeword_sensitivity
                .as_deref()
                .and_then(WakewordSensitivity::parse)
                .unwrap_or(WakewordSensitivity::Medium)
        };

        let voice_patterns = load_voice_patterns_from(&file_config);

        Ok(Config {
            api_key,
            timeout,
            model,
            delay: args.delay,
            output_mode,
            wakeword_sensitivity,
            voice_patterns,
        })
    }
}

/// Load voice patterns from config, with legacy migration.
pub fn load_voice_patterns_from(file_config: &ConfigFile) -> Vec<VoicePattern> {
    if !file_config.voice_pattern.is_empty() {
        return file_config
            .voice_pattern
            .iter()
            .map(|entry| {
                let path = entry
                    .path
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| voice_pattern_path_for_name(&entry.name));
                let action = VoiceAction::parse(&entry.action)
                    .unwrap_or(VoiceAction::Transcribe);
                VoicePattern {
                    name: entry.name.clone(),
                    path,
                    action,
                }
            })
            .collect();
    }

    // Legacy migration: check old single-wakeword fields
    let legacy_path = file_config
        .wakeword_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(legacy_wakeword_path);

    if legacy_path.exists() {
        let name = file_config
            .wakeword_name
            .clone()
            .unwrap_or_else(|| "hey voclip".to_string());
        return vec![VoicePattern {
            name,
            path: legacy_path,
            action: VoiceAction::Transcribe,
        }];
    }

    Vec::new()
}

// --- Path helpers ---

/// Path for a voice pattern .rpw file, derived from its name.
pub fn voice_pattern_path_for_name(name: &str) -> PathBuf {
    let config_dir = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let slug = name.to_lowercase().replace(' ', "_");
    config_dir
        .join("voclip")
        .join("voice_patterns")
        .join(format!("{slug}.rpw"))
}

/// Legacy single wakeword path (for migration).
pub fn legacy_wakeword_path() -> PathBuf {
    let config_dir = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("voclip").join("wakeword.rpw")
}

// --- Save/remove voice patterns ---

pub fn save_voice_pattern(name: &str, action: &VoiceAction) -> Result<(), VoclipError> {
    let mut config = ConfigFile::load();

    // Remove existing entry with same name
    config.voice_pattern.retain(|e| e.name != name);

    config.voice_pattern.push(VoicePatternEntry {
        name: name.to_string(),
        action: action.to_config_string(),
        path: None,
    });

    config.save()
}

pub fn remove_voice_pattern(name: &str) -> Result<bool, VoclipError> {
    let mut config = ConfigFile::load();
    let before = config.voice_pattern.len();
    config.voice_pattern.retain(|e| e.name != name);
    let removed = config.voice_pattern.len() < before;

    if removed {
        config.save()?;
    }

    // Also delete the .rpw file if it exists
    let path = voice_pattern_path_for_name(name);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }

    Ok(removed)
}

pub fn list_voice_patterns() {
    let config = ConfigFile::load();
    let patterns = load_voice_patterns_from(&config);

    if patterns.is_empty() {
        println!("No voice patterns configured.");
        println!("Use --train-wakeword to train a wake word.");
        println!("Use --train-command to train a command word.");
        return;
    }

    let wake_words: Vec<_> = patterns
        .iter()
        .filter(|p| p.action == VoiceAction::Transcribe)
        .collect();
    let commands: Vec<_> = patterns
        .iter()
        .filter(|p| p.action != VoiceAction::Transcribe)
        .collect();

    println!("Voice patterns:\n");

    if !wake_words.is_empty() {
        println!("  Wake word:");
        for p in &wake_words {
            let status = if p.path.exists() { "trained" } else { "not trained" };
            println!("    {:<20} {:<18} ({})", format!("\"{}\"", p.name), p.action, status);
        }
    }

    if !commands.is_empty() {
        if !wake_words.is_empty() {
            println!();
        }
        println!("  Command words:");
        for p in &commands {
            let status = if p.path.exists() { "trained" } else { "not trained" };
            println!("    {:<20} {:<18} ({})", format!("\"{}\"", p.name), p.action, status);
        }
    }
}

// --- Other settings ---

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
