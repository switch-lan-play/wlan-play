use serde_derive::Deserialize;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Agent {
    pub url: Url,
    pub after_connected: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub agent: Agent
}
