use super::SSHEventHandler;
use crate::AppResult;
use anyhow::{anyhow, Result};
use russh::server::Handle;
use russh::ChannelId;
use std::fmt::Debug;
use tokio::sync::mpsc::{self, Receiver};

#[derive(Clone)]
pub struct SSHWriterProxy {
    flushing: bool,
    channel_id: ChannelId,
    handle: Handle,
    // The sink collects the data which is finally flushed to the handle.
    pub sink: Vec<u8>,
}

impl Debug for SSHWriterProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SSHWriterProxy")
            .field("flushing", &self.flushing)
            .field("channel_id", &self.channel_id)
            .field("sink", &self.sink)
            .finish()
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for SSHWriterProxy {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushing = true;
        Ok(())
    }
}

impl SSHWriterProxy {
    pub fn new(channel_id: ChannelId, handle: Handle) -> Self {
        Self {
            flushing: false,
            channel_id,
            handle,
            sink: vec![],
        }
    }

    pub async fn send(&mut self) -> std::io::Result<usize> {
        if !self.flushing {
            return Ok(0);
        }

        let data_length = self.sink.len();

        if self
            .handle
            .data(self.channel_id, self.sink.clone().into())
            .await
            .is_err()
        {
            let _ = self.handle.close(self.channel_id).await;
        }

        self.sink.clear();
        self.flushing = false;
        Ok(data_length)
    }

    pub async fn exit(&mut self) {
        let _ = self.handle.eof(self.channel_id).await;
        let _ = self.handle.close(self.channel_id).await;
    }
}

#[derive(Debug)]
pub struct AppChannel {
    state: AppChannelState,
}

#[derive(Debug)]
enum AppChannelState {
    AwaitingPty,
    Ready { stdin: mpsc::Sender<Vec<u8>> },
}

impl AppChannel {
    pub fn new() -> Self {
        let state = AppChannelState::AwaitingPty;
        Self { state }
    }

    pub async fn data(&mut self, data: &[u8]) -> Result<()> {
        let AppChannelState::Ready { stdin } = &mut self.state else {
            return Err(anyhow!("pty hasn't been allocated yet"));
        };

        stdin
            .send(data.to_vec())
            .await
            .map_err(|_| anyhow!("lost ui"))?;

        Ok(())
    }

    pub async fn pty_request(&mut self) -> AppResult<Receiver<Vec<u8>>> {
        let AppChannelState::AwaitingPty = &mut self.state else {
            return Err(anyhow!("pty has been already allocated"));
        };

        let (stdin_tx, stdin_rx) = mpsc::channel(1);
        self.state = AppChannelState::Ready { stdin: stdin_tx };

        Ok(stdin_rx)
    }

    pub async fn window_change_request(&mut self, width: u32, height: u32) -> Result<()> {
        let AppChannelState::Ready { stdin } = &mut self.state else {
            return Err(anyhow!("pty hasn't been allocated yet"));
        };

        let width = width.min(255);
        let height = height.min(255);

        stdin
            .send(vec![SSHEventHandler::CMD_RESIZE, width as u8, height as u8])
            .await
            .map_err(|_| anyhow!("lost ui"))?;

        Ok(())
    }
}
