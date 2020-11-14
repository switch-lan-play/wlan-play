use anyhow::{anyhow, Result};
use super::{Url, Connection};
use thrussh::{client, ChannelMsg};
use thrussh_keys::key;
use std::sync::Arc;

pub struct SshConnection {
    handle: client::Handle,
    channel: client::Channel,
}

#[async_trait::async_trait]
impl Connection for SshConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.channel.data(data).await
    }
    async fn recv(&mut self) -> Option<Vec<u8>> {
        match self.channel.wait().await {
            Some(ChannelMsg::Data{ data }) => {
                let mut dat = Vec::with_capacity(data.len());
                data.write_all_from(0, &mut dat).unwrap();
                Some(dat)
            },
            _ => None
        }
    }
}

struct Handler;
impl client::Handler for Handler {
    type FutureUnit = futures::future::Ready<Result<(Self, client::Session), anyhow::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, bool), anyhow::Error>>;
 
    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }
    fn finished(self, session: client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }
    fn check_server_key(self, server_public_key: &key::PublicKey) -> Self::FutureBool {
        log::info!("check_server_key: {:?}", server_public_key);
        self.finished_bool(true)
    }
 }

pub async fn connect(url: &Url) -> Result<Box<dyn Connection>> {
    assert_eq!(url.scheme(), "ssh");

    let config = client::Config::default();
    let config = Arc::new(config);
    let addr = format!(
        "{}:{}",
        url.host_str().ok_or(anyhow!("Host is empty"))?,
        url.port_or_known_default().unwrap_or(22)
    );
    if url.path().len() == 0 {
        Err(anyhow!("Command is empty"))?;
    }
    let cmd = &url.path()[1..];
    let mut handle  = client::connect(config, addr, Handler).await?;

    if !handle.authenticate_password(
        url.username(),
        url.password().ok_or(anyhow!("Password is empty"))?,
    ).await? {
        Err(anyhow!("Failed to login with {}", url.username()))?;
    }

    let mut channel = handle.channel_open_session().await?;
    channel.exec(true, cmd).await?;
    while let Some(msg) = channel.wait().await {
        log::debug!("exec {:?}", msg);
        if let ChannelMsg::Success = msg {
            log::info!("Get shell");
            break
        }
    }

    let mut conn = SshConnection {
        handle,
        channel,
    };

    conn.send(&b"echo 123\n"[..]).await?;
    let a = conn.recv().await;
    log::info!("{:?}", std::str::from_utf8(&a.unwrap()));

    Ok(Box::new(conn))
}
