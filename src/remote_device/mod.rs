use crate::Result;
use crate::agent::BoxAgent;
use std::future::Future;

pub struct RemoteDevice<F> {
    control: BoxAgent,
    send: Option<BoxAgent>,
    recv: Option<BoxAgent>,
    agent_factory: F,
    device_name: String,
}

impl<F, Fut> RemoteDevice<F>
where
    F: Fn() -> Fut,
    Fut: Future<Output=Result<BoxAgent>>
{
    /// make a remote device over agent. the device must be monitor mode.
    pub async fn new<D: Into<String>>(agent_factory: F, device_name: D) -> Result<RemoteDevice<F>>
    {
        let control = agent_factory().await?;

        Ok(RemoteDevice {
            control,
            send: None,
            recv: None,
            agent_factory,
            device_name: device_name.into(),
        })
    }
}
