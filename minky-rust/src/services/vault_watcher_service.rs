//! Vault watcher service — file-system event loop for vault ingestion.
//!
//! Watches one or more vault root directories using `notify-debouncer-full` and
//! automatically ingests new or modified `.md` files through the same pipeline
//! as `POST /api/vault/ingest`.

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::oneshot;

use crate::services::embedding_service::EmbeddingService;

// ── Handle ────────────────────────────────────────────────────────────────────

/// A lightweight handle returned by [`VaultWatcherService::start`].
///
/// Dropping this handle does **not** stop the watcher; call [`stop`] explicitly
/// so that the shutdown is intentional and observable in logs.
///
/// [`stop`]: VaultWatcherHandle::stop
pub struct VaultWatcherHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl VaultWatcherHandle {
    /// Signal the background watcher task to shut down.
    ///
    /// The call is non-blocking: it sends a shutdown signal and returns
    /// immediately.  The background task acknowledges the signal and exits
    /// asynchronously.  Calling `stop` on an already-stopped watcher is safe
    /// and has no effect.
    pub fn stop(self) {
        // oneshot::Sender::send returns Err only when the receiver is already
        // dropped, which is not an error condition from the caller's point of
        // view (the task has already exited).
        let _ = self.shutdown_tx.send(());
    }
}

// ── Service ───────────────────────────────────────────────────────────────────

/// Watches one or more vault root directories and ingests new/modified `.md`
/// files into MinKy automatically.
///
/// # Lifecycle
///
/// ```text
/// VaultWatcherService::new(pool, embedding_service)
///     └─ .start(roots, user_id) -> VaultWatcherHandle
///             └─ .stop()
/// ```
pub struct VaultWatcherService {
    pool: PgPool,
    embedding_service: Arc<EmbeddingService>,
}

impl VaultWatcherService {
    /// Create a new service instance.
    ///
    /// The service does not begin watching until [`start`] is called.
    ///
    /// [`start`]: VaultWatcherService::start
    pub fn new(pool: PgPool, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            pool,
            embedding_service,
        }
    }

    /// Validate `roots` and spawn a background watcher task.
    ///
    /// Returns a [`VaultWatcherHandle`] that can be used to stop the watcher
    /// later.  All roots are validated with the shared [`validate_path`] helper
    /// before the background task is spawned; the first invalid root causes an
    /// immediate error.
    ///
    /// # Errors
    ///
    /// Returns an error if any element of `roots` fails [`validate_path`]
    /// (e.g. relative path, path-traversal component, or non-existent path).
    ///
    /// [`validate_path`]: crate::services::vault_common::validate_path
    pub async fn start(
        self,
        roots: Vec<PathBuf>,
        user_id: i32,
    ) -> anyhow::Result<VaultWatcherHandle> {
        use crate::services::vault_common::validate_path;
        use notify_debouncer_full::{
            new_debouncer,
            notify::{EventKind, RecursiveMode, Watcher},
            DebounceEventResult,
        };
        use std::time::Duration;

        // Validate all roots before starting so we fail fast with a clear error.
        for root in &roots {
            validate_path(root.to_str().unwrap_or(""))
                .map_err(|e| anyhow::anyhow!("Invalid watch root {:?}: {}", root, e))?;
        }

        let pool = self.pool;
        let embedding_service = Arc::clone(&self.embedding_service);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        // Channel for events from the sync notify callback to the async handler.
        // Capacity 64 allows bursts without blocking the watcher thread.
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel::<Vec<PathBuf>>(64);

        tokio::spawn(async move {
            // Create the debouncer in a blocking thread since the notify crate
            // is synchronous.  We return the debouncer so it stays alive for
            // the duration of the watch loop.
            let debouncer_result = tokio::task::spawn_blocking({
                let roots = roots.clone();
                let event_tx = event_tx.clone();
                move || {
                    let mut debouncer = new_debouncer(
                        Duration::from_secs(2),
                        None,
                        move |result: DebounceEventResult| {
                            if let Ok(events) = result {
                                let paths: Vec<PathBuf> = events
                                    .into_iter()
                                    .filter_map(|e| {
                                        if matches!(
                                            e.event.kind,
                                            EventKind::Create(_) | EventKind::Modify(_)
                                        ) {
                                            Some(e.event.paths)
                                        } else {
                                            None
                                        }
                                    })
                                    .flatten()
                                    .collect();
                                if !paths.is_empty() {
                                    let _ = event_tx.try_send(paths);
                                }
                            }
                        },
                    )?;

                    for root in &roots {
                        debouncer.watcher().watch(root, RecursiveMode::Recursive)?;
                    }

                    // Return the debouncer so it is kept alive while the loop runs.
                    Ok::<_, notify_debouncer_full::notify::Error>(debouncer)
                }
            })
            .await;

            match debouncer_result {
                Ok(Ok(mut _debouncer)) => {
                    tracing::info!(
                        root_count = roots.len(),
                        user_id,
                        "VaultWatcher started",
                    );

                    loop {
                        tokio::select! {
                            Some(paths) = event_rx.recv() => {
                                for path in paths {
                                    use crate::services::vault_common::is_safe_md_path;

                                    if !is_safe_md_path(&path) {
                                        tracing::debug!(
                                            "VaultWatcher: skipping non-md or unsafe path {:?}",
                                            path
                                        );
                                        continue;
                                    }

                                    match crate::routes::vault::ingest_single_file(
                                        &pool,
                                        &path,
                                        user_id,
                                        &embedding_service,
                                    )
                                    .await
                                    {
                                        Ok(Some(id)) => tracing::info!(
                                            "VaultWatcher: ingested {:?} → {}",
                                            path,
                                            id
                                        ),
                                        Ok(None) => tracing::debug!(
                                            "VaultWatcher: skipped {:?}",
                                            path
                                        ),
                                        Err(e) => tracing::warn!(
                                            "VaultWatcher: ingest error for {:?}: {}",
                                            path,
                                            e
                                        ),
                                    }
                                }
                            }
                            _ = &mut shutdown_rx => {
                                tracing::info!("VaultWatcher stopped");
                                break;
                            }
                        }
                    }
                }
                Ok(Err(e)) => tracing::error!("VaultWatcher: failed to create file watcher: {}", e),
                Err(e) => tracing::error!("VaultWatcher: spawn_blocking failed: {}", e),
            }
        });

        Ok(VaultWatcherHandle { shutdown_tx })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_stop_does_not_panic() {
        let (tx, _rx) = oneshot::channel::<()>();
        let handle = VaultWatcherHandle { shutdown_tx: tx };
        // Dropping _rx before calling stop means the receiver is gone, yet
        // stop() must not panic.
        handle.stop();
    }

    #[test]
    fn invalid_root_relative_path_rejected() {
        use crate::services::vault_common::validate_path;
        assert!(validate_path("relative/path").is_err());
    }

    #[test]
    fn invalid_root_dotdot_rejected() {
        use crate::services::vault_common::validate_path;
        assert!(validate_path("/tmp/../etc").is_err());
    }

    #[test]
    fn event_paths_filter_non_md() {
        use crate::services::vault_common::is_safe_md_path;

        let dir = tempfile::tempdir().unwrap();

        // A plain .md file should be safe.
        let md_path = dir.path().join("test.md");
        std::fs::write(&md_path, "# hello").unwrap();
        assert!(is_safe_md_path(&md_path));

        // A .txt file must not pass the filter.
        let txt_path = dir.path().join("test.txt");
        std::fs::write(&txt_path, "hello").unwrap();
        assert!(!is_safe_md_path(&txt_path));
    }

    #[test]
    fn event_paths_filter_symlinks() {
        use crate::services::vault_common::is_safe_md_path;

        let dir = tempfile::tempdir().unwrap();
        let real_md = dir.path().join("real.md");
        std::fs::write(&real_md, "# real").unwrap();

        // A regular .md file passes.
        assert!(is_safe_md_path(&real_md));

        // A symlink to a .md file must be rejected.
        let link_path = dir.path().join("link.md");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real_md, &link_path).unwrap();
            assert!(!is_safe_md_path(&link_path));
        }
    }
}
