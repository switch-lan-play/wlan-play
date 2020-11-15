use anyhow::{anyhow, Result};
use tokio::io::ReadBuf;
use super::{Url, AsyncStream, AsyncRead, AsyncWrite, BoxAsyncStream};
use thrussh::{client, ChannelMsg};
use thrussh_keys::key;
use std::{sync::Arc, pin::Pin, task::{Context, Poll}, io};
use futures::{Future, ready, pin_mut};

pub struct SshConnection {
    _handle: client::Handle,
    channel: client::Channel,
    read_buf: Option<Vec<u8>>,
}

impl AsyncRead for SshConnection {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        loop {
            let t = self.read_buf.take();
            self.read_buf = match t {
                Some(mut b) => {
                    let new_b = b.split_off(buf.remaining().min(b.len()));
                    buf.put_slice(&b);
                    self.read_buf = Some(new_b);
                    return Poll::Ready(Ok(()))
                },
                None => {
                    let fut = self.channel.wait();
                    pin_mut!(fut);
                    match ready!(fut.poll(cx)) {
                        Some(ChannelMsg::Data{ data }) => {
                            let mut dat = Vec::with_capacity(data.len());
                            data.write_all_from(0, &mut dat).unwrap();
                            Some(dat)
                        }
                        _ => return Poll::Ready(Err(io::ErrorKind::InvalidData.into()))
                    }
                },
            }
        }
    }
}

impl AsyncWrite for SshConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let fut = self.channel.data(buf);
        pin_mut!(fut);
        let r = ready!(fut.poll(cx));
        Poll::Ready(r
            .map(|_| buf.len())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        )
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[async_trait::async_trait]
impl AsyncStream for SshConnection {
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

pub async fn connect(url: &Url) -> Result<BoxAsyncStream> {
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

    Ok(Box::new(SshConnection {
        _handle: handle,
        channel,
        read_buf: None,
    }))
}
