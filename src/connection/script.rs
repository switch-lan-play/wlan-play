use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result, Error};
use super::Connection;
use crate::utils::timeout::{TimeoutExt, DEFAULT_TIMEOUT};
use rhai::{Dynamic, Engine, EvalAltResult, RegisterFn, RegisterResultFn};
use tokio::task;
use futures::executor::block_on;
use tokio::prelude::*;

#[derive(Clone)]
pub struct ScriptConnection(Arc<Mutex<Connection>>);

impl ScriptConnection {
    fn send(&mut self, command: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut conn = self.0.lock().unwrap();
        block_on(async {
            conn.write_all(command.as_bytes()).await?;
            conn.flush().await?;
            Result::<(), Error>::Ok(())
        })
            .map(Into::into)
            .map_err(|_| "Failed to send".into())
    }
    fn recv(&mut self) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut buf = [0u8;1024];
        let mut conn = self.0.lock().unwrap();
        block_on(conn.read(&mut buf))
            .map(|size| String::from_utf8_lossy(&buf[0..size]).into_owned().into())
            .map_err(|_| "Failed to recv".into())
    }
    fn read_line(&mut self) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut conn = self.0.lock().unwrap();
        block_on(conn.read_line_timeout(DEFAULT_TIMEOUT))
            .map(Into::into)
            .map_err(|_| "Failed to read_line".into())
    }
    fn into_inner(self) -> Connection {
        match Arc::try_unwrap(self.0) {
            Ok(c) => c.into_inner().unwrap(),
            Err(_) => panic!("Failed to unwrap"),
        }
    }
}

pub async fn run_script(conn: Connection, script: String) -> Result<Connection> {
    task::spawn_blocking(move || {
        let conn = ScriptConnection(Arc::new(Mutex::new(conn)));
        let get_conn = conn.clone();
        let mut engine = Engine::new();
    
        engine
            .on_print(|x| log::debug!("Script: {}", x))
            .on_debug(|x| log::debug!("Script: {}", x))
            .register_type::<ScriptConnection>()
            .register_result_fn("send", ScriptConnection::send)
            .register_result_fn("recv", ScriptConnection::recv)
            .register_result_fn("read_line", ScriptConnection::read_line)
            .register_fn("conn", move || get_conn.clone());
    
        let mut ast = engine.compile("let conn = conn();")?;
        ast += engine.compile(&script)?;
    
        engine.eval_ast::<()>(&ast).map_err(|e| anyhow!("Failed to run script: {:?}", e))?;
        drop(engine);
    
        Ok(conn.into_inner())
    }).await?
}
