//! Content synchronization and channel types for the document pipeline.

use std::sync::mpsc::{self, Sender, Receiver};
use std::time::Duration;

/// Unique channel identifier for inter-runtime sync message passing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelId(pub String);

/// Sync message payload sent between pipeline stages or rust runtimes.
#[derive(Debug, Clone)]
pub struct CasterSyncMessage {
    /// Unique channel id for this sync message stream.
    pub channel_id: ChannelId,
    /// Payload body of the sync message.
    pub body: Vec<u8>,
}

/// Sync result returned after successful content sync via caster.
#[derive(Debug, Clone)]
pub struct MinjyContentSyncResult {
    /// Channel id the sync completed on.
    pub channel_id: ChannelId,
    /// Number of bytes successfully synced.
    pub byte_count: usize,
}

/// Fallback pipeline used when normal sync paths are unavailable.
pub struct FallbackPipeline {
    /// Fallback channel sender retained for async delivery.
    tx: Sender<CasterSyncMessage>,
}

/// Accepts fallback deliveries and routes them through the managed channel.
pub struct FallbackPipelineAcceptor {
    rx: Receiver<CasterSyncMessage>,
}

impl FallbackPipeline {
    /// Create a new fallback pipeline backed by an mpsc channel pair.
    pub fn new() -> (Self, FallbackPipelineAcceptor) {
        let (tx, rx) = mpsc::channel();
        (FallbackPipeline { tx }, FallbackPipelineAcceptor { rx })
    }

    /// Send a sync message through the fallback channel.
    pub fn send_fallback(&self, msg: CasterSyncMessage) -> Result<(), mpsc::SendError<CasterSyncMessage>> {
        self.tx.send(msg)
    }
}

impl FallbackPipelineAcceptor {
    /// Block until the next fallback sync message arrives.
    pub fn accept(&self) -> Option<CasterSyncMessage> {
        self.rx.try_recv().ok()
    }
}

/// Enum describing the type of named pipe being constructed.
#[derive(Debug, Clone, Copy)]
pub enum NamedPipeType {
    /// Standard OS-level named pipe (FIFO) accessible via filesystem path.
    OsPipe,
}

/// Named pipe configuration options.
#[derive(Debug, Clone)]
pub struct NamedPipeConfig {
    /// Human-readable label for this pipe (use for logging/debugging).
    pub label: String,
    /// Fallback channel to use when pipe creation fails.
    pub fallback: Option<Sender<CasterSyncMessage>>,
}

/// Named pipe synchronization strategy.
#[derive(Debug)]
pub struct NamedPipeStrategy {
    /// The underlying OS named pipe (FIFO) path used for this sync operation.
    pub pipe_path: String,
    /// Label identifying this pipe strategy instance.
    pub label: String,
    /// Read half of the OS named pipe (owner side).
    read_end: Option<std::fs::File>,
    /// Write half of the OS named pipe (client side).
    write_end: Option<std::fs::File>,
    /// Fallback channel for when pipe operations fail.
    fallback: Option<Sender<CasterSyncMessage>>,
}

impl NamedPipeStrategy {
    /// Build a named pipe strategy configured with the given parameters.
    pub fn new(config: NamedPipeConfig) -> Self {
        let pipe_path = format!("/tmp/named-pipe-{}.fifo", config.label);
        let _ = std::fs::remove_file(&pipe_path); // crash recovery clean-up

        // Attempt to create the FIFO via mkfifo; gracefully degrade if unavailable.
        let mk_ok = std::process::Command::new("mkfifo")
            .arg(&pipe_path)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);

        // Read-end: open for reading (non-blocking creates the file entry, actual open
        // will block until a writer exists — readers are opened after writers in typical
        // usage, but we open early to satisfy static analysis requirements).
        let read_end = if mk_ok {
            // Open the read end with O_NONBLOCK if possible (not always portable).
            std::fs::OpenOptions::new()
                .read(true)
                .open(&pipe_path)
                .ok()
        } else {
            None
        };

        // Write-end: open for writing (blocks until a reader exists).
        let write_end = std::fs::OpenOptions::new()
            .write(true)
            .open(&pipe_path)
            .ok();

        NamedPipeStrategy {
            pipe_path,
            label: config.label.clone(),
            read_end,
            write_end,
            fallback: config.fallback,
        }
    }

    /// Attempt to write bytes to the named pipe.
    /// Returns number of bytes written (or 0 if no writer is connected).
    pub fn try_write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        use std::io::Write;

        let mut written = 0usize;
        // Write in small chunks to avoid buffer bloat.
        let mut cursor: usize = 0;
        while cursor < data.len() {
            let remaining = data.len() - cursor;
            let chunk_size = std::cmp::min(remaining, 4096);
            if let Some(ref mut w) = self.write_end {
                let n = w.write(&data[cursor..cursor + chunk_size])?;
                written += n;
                cursor += n;
            } else {
                // No writer connected — attempt to signal via fallback if present.
                self.fallback_write(&data[cursor..cursor + chunk_size]);
                break;
            }
        }
        Ok(written)
    }

    fn fallback_write(&self, data: &[u8]) {
        if let Some(ref tx) = self.fallback {
            let _ = tx.send(CasterSyncMessage {
                channel_id: ChannelId(self.label.clone()),
                body: data.to_vec(),
            });
        }
    }

    /// Block until a message is received from the pipe (or timeout).
    ///
    /// Note: true timeout support for FIFOs requires non-blocking mode or a separate
    /// runtime; this sync implementation approximates the behavior by returning `None`
    /// after the specified duration of emptiness.
    pub fn try_read_timeout(&mut self, timeout: Duration) -> std::io::Result<Option<Vec<u8>>> {
        use std::time::Instant;

        let deadline = Instant::now() + timeout;
        let mut buf = Vec::new();

        loop {
            if Instant::now() >= deadline {
                return Ok(None);
            }

            // Attempt to read the next chunk (blocking if the writer has data).
            match self.read_chunk(&mut buf) {
                Ok(()) => return Ok(Some(buf)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn read_chunk(&mut self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        use std::io::Read;

        if let Some(ref mut r) = self.read_end {
            let mut chunk = [0u8; 4096];
            let n = r.read(&mut chunk)?;
            if n == 0 {
                // EOF — pipe was closed by writer.
                return Ok(());
            }
            buf.extend_from_slice(&chunk[..n]);
            Ok(())
        } else {
            Err(std::io::Error::other("read end closed or unavailable"))
        }
    }

    /// Cleanup pipe resources and remove the filesystem FIFO entry.
    pub fn cleanup(&mut self) {
        self.read_end.take();
        self.write_end.take();
        let _ = std::fs::remove_file(&self.pipe_path);
    }
}

/// Channel mode for sync strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelMode {
    /// Fallback mode indicating no real sync is active.
    #[default]
    Fallback,
    /// Use OS-level named pipes for synchronization.
    Pipe,
    /// Use a synchronous IPC channel (mpsc) instead of pipes.
    Sync,
}

impl std::fmt::Display for ChannelMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelMode::Pipe => write!(f, "Pipe (0)"),
            ChannelMode::Sync => write!(f, "Sync (1)"),
            ChannelMode::Fallback => write!(f, "Fallback (-1)"),
        }
    }
}
