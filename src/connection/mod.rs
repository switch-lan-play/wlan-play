use anyhow::{Result, anyhow};
pub use url::Url;
pub use traits::Connection;

mod ssh;
mod traits;

pub async fn connect(url: &Url) -> Result<Box<dyn Connection>> {
    Ok(match url.scheme() {
        "ssh" => {
            ssh::connect(url).await
        }
        _ => {
            Err(anyhow!("{} not support!", url.scheme()))?
        }
    }?)
}
