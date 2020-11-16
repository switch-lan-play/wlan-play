use crate::Result;
pub use traits::{Agent, BoxAgent, Commander};
use crate::connection::Connection;
use serde_derive::Deserialize;

mod traits;
mod linux;

#[derive(Deserialize, Debug)]
pub enum Platform {
    Linux,
}


pub async fn from_connection(platform: &Platform, conn: Connection) -> Result<BoxAgent> {
    let mut agent = match platform {
        Platform::Linux => {
            linux::from_connection(conn).await?
        }
    };
    agent.check().await?;
    Ok(agent)
}
