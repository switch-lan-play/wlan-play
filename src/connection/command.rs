use anyhow::Result;
use super::{AsyncStream, BoxAsyncStream};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::{ pin::Pin, task::{Context, Poll}, io};

pub struct CommandConnection {
    child: Child,
}

impl CommandConnection {
    fn stdin(&mut self) -> &mut ChildStdin {
        self.child.stdin.as_mut().unwrap()
    }
    fn stdout(&mut self) -> &mut ChildStdout {
        self.child.stdout.as_mut().unwrap()
    }
}

impl AsyncRead for CommandConnection {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.stdout()).poll_read(cx, buf)
    }
}

impl AsyncWrite for CommandConnection {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        Pin::new(self.stdin()).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(self.stdin()).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(self.stdin()).poll_shutdown(cx)
    }
}

#[async_trait::async_trait]
impl AsyncStream for CommandConnection {
}

pub async fn connect((command, args): (&String, &[String])) -> Result<BoxAsyncStream> {
    let child = Command::new(command)
        .args(args)
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    Ok(Box::new(CommandConnection{
        child,
    }))
}
