use clap::Parser;

use crate::error::VoclipError;

#[derive(Parser, Debug)]
#[command(name = "voclip", about = "Voice to clipboard — speak and paste")]
pub struct Args {
    /// Silence timeout in seconds
    #[arg(long, default_value_t = 3)]
    pub timeout: u32,

    /// Language code or "multi" for auto-detect
    #[arg(long, default_value = "multi")]
    pub language: String,
}

pub struct Config {
    pub api_key: String,
    pub timeout: u32,
    pub language: String,
}

impl Config {
    pub fn load() -> Result<Self, VoclipError> {
        let args = Args::parse();

        // Try .env first (non-fatal if missing)
        let _ = dotenvy::dotenv();

        let api_key = std::env::var("ASSEMBLYAI_API_KEY")
            .map_err(|_| VoclipError::MissingApiKey)?;

        Ok(Config {
            api_key,
            timeout: args.timeout,
            language: args.language,
        })
    }
}
