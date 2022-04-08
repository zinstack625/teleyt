use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub vid_format: String,
    pub aud_format: String,
    pub redis_address: Option<String>,
    pub telegram_token: String,
}
