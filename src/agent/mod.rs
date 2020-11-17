use crate::Result;
pub use traits::*;
use crate::connection::Connection;
use serde_derive::Deserialize;
pub use linux::LinuxAgent;

mod traits;
mod linux;

#[derive(Deserialize, Debug)]
pub enum Platform {
    Linux,
}

pub async fn from_connection(platform: &Platform, conn: Connection) -> Result<BoxAgent> {
    let mut agent = match platform {
        Platform::Linux => {
            Box::new(LinuxAgent::new(conn).await?)
        }
    };
    agent.check().await?;
    Ok(agent)
}
