use anyhow::{Result, anyhow};
pub use url::Url;
pub use traits::{Connection, BoxConnection};
pub use script::run_script;

mod ssh;
mod traits;
mod script;

pub async fn connect(url: &Url) -> Result<BoxConnection> {
    Ok(match url.scheme() {
        "ssh" => {
            ssh::connect(url).await
        }
        _ => {
            Err(anyhow!("{} not support!", url.scheme()))?
        }
    }?)
}
