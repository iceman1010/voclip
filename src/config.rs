use clap::Parser;

use crate::error::VoclipError;

#[derive(Parser, Debug)]
#[command(name = "voclip", about = "Voice to clipboard — speak and paste")]
pub struct Args {
    /// Check for updates and self-update if a newer version is available
    #[arg(long)]
    pub update: bool,

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
    pub fn load(args: &Args) -> Result<Self, VoclipError> {
        let _ = dotenvy::dotenv();

        let api_key =
            std::env::var("ASSEMBLYAI_API_KEY").map_err(|_| VoclipError::MissingApiKey)?;

        Ok(Config {
            api_key,
            timeout: args.timeout,
            language: args.language.clone(),
        })
    }
}
