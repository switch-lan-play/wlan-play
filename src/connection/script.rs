use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};
use super::BoxConnection;
use rhai::{Dynamic, Engine, EvalAltResult, RegisterFn, RegisterResultFn};
use tokio::task;
use tokio::prelude::*;
use futures::executor::block_on;

#[derive(Clone)]
pub struct ScriptConnection(Arc<Mutex<BoxConnection>>);

impl ScriptConnection {
    fn send(&mut self, command: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut conn = self.0.lock().unwrap();
        match block_on(conn.write_all(command.as_bytes())) {
            Ok(_) => {
                Ok(().into())
            }
            Err(_) => {
                Err("Failed to send".into())
            }
        }
    }
    fn recv(&mut self) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut buf = [0u8;1024];
        let mut conn = self.0.lock().unwrap();
        match block_on(conn.read(&mut buf)) {
            Ok(size) => {
                Ok(String::from_utf8_lossy(&buf[0..size]).into_owned().into())
            }
            Err(_) => {
                Err("Failed to recv".into())
            }
        }
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
            .register_result_fn("recv", ScriptConnection::recv)
            .register_fn("conn", move || get_conn.clone());
    
        let mut ast = engine.compile("let conn = conn();")?;
        ast += engine.compile(&script)?;
    
        engine.eval_ast::<()>(&ast).map_err(|e| anyhow!("Failed to run script: {:?}", e))?;
        drop(engine);
    
        Ok(conn.into_inner())
    }).await?
}
