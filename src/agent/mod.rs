use crate::Result;
pub use traits::Agent;
use crate::connection::Connection;
use serde_derive::Deserialize;

mod traits;


#[derive(Deserialize, Debug)]
pub enum Platform {
    Linux,
}


pub async fn from_connection(platform: Platform, conn: Box<dyn Connection>) -> Result<Box<dyn Agent>> {
    unimplemented!()
}
