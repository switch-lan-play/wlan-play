use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};
use super::BoxConnection;
use rhai::{Dynamic, Engine, EvalAltResult, RegisterFn, RegisterResultFn};
use tokio::task;
use futures::executor::block_on;

#[derive(Clone)]
pub struct ScriptConnection(Arc<Mutex<BoxConnection>>);

impl ScriptConnection {
    fn send(&mut self, command: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        match block_on(self.0.lock().unwrap().send(command.as_bytes())) {
            Ok(_) => {
                Ok(().into())
            }
            Err(_) => {
                Err("Failed to send".into())
            }
        }
    }
    fn recv(&mut self) -> String {
        block_on(self.0.lock().unwrap().recv()).map(|v| String::from_utf8_lossy(&v).into_owned()).unwrap_or_default()
    }
    fn into_inner(self) -> BoxConnection {
        match Arc::try_unwrap(self.0) {
            Ok(c) => c.into_inner().unwrap(),
            Err(_) => panic!("Failed to unwrap"),
        }
    }
}

pub async fn run_script(conn: BoxConnection, script: String) -> Result<BoxConnection> {
    task::spawn_blocking(move || {
        let conn = ScriptConnection(Arc::new(Mutex::new(conn)));
        let get_conn = conn.clone();
        let mut engine = Engine::new();
    
        engine
            .on_print(|x| log::info!("Script: {}", x))
            .on_debug(|x| log::info!("Script: {}", x))
            .register_type::<ScriptConnection>()
            .register_result_fn("send", ScriptConnection::send)
            .register_fn("recv", ScriptConnection::recv)
            .register_fn("conn", move || get_conn.clone());
    
        let mut ast = engine.compile("let conn = conn();")?;
        ast += engine.compile(&script)?;
    
        engine.eval_ast::<()>(&ast).map_err(|e| anyhow!("Failed to run script: {:?}", e))?;
        drop(engine);
    
        Ok(conn.into_inner())
    }).await?
}
