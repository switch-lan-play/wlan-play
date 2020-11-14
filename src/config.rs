use serde_derive::Deserialize;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Agent {
    pub url: Url
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub agent: Agent
}
