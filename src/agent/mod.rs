use crate::Result;
pub use traits::Agent;
use crate::connection::Connection;

mod traits;

pub async fn from_connection(conn: Box<dyn Connection>) -> Result<Box<dyn Agent>> {
    unimplemented!()
}
