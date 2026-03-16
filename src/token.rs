use serde::Deserialize;

use crate::error::VoclipError;

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

pub async fn fetch_token(api_key: &str) -> Result<String, VoclipError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://streaming.assemblyai.com/v3/token?expires_in_seconds=600")
        .header("Authorization", api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(VoclipError::TokenFetch(format!("{status}: {body}")));
    }

    let token_resp: TokenResponse = resp.json().await?;
    Ok(token_resp.token)
}
