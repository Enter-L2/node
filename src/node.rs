//! Core node implementation

use anyhow::{Context, Result};
use futures::future::join_all;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use crate::{
    config::NodeConfig,
    metrics::MetricsServer,
    p2p::P2PManager,
    rpc::RpcServer,
    storage::Storage,
    sync::SyncManager,
    verification::VerificationManager,
};

pub struct Node {
    config: NodeConfig,
    storage: Arc<Storage>,
    sync_manager: Arc<SyncManager>,
    rpc_server: Option<Arc<RpcServer>>,
    p2p_manager: Arc<P2PManager>,
    metrics_server: Option<Arc<MetricsServer>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl Node {
    pub async fn new(config: NodeConfig) -> Result<Self> {
        info!("Initializing Enter L2 Node");

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(4);

        let storage = Arc::new(
            Storage::new(&config.data_dir, &config.database)
                .await
                .context("failed to initialize storage")?,
        );

        let verification_manager = Arc::new(
            VerificationManager::new(&config.verification)
                .await
                .context("failed to initialize verification manager")?,
        );

        let p2p_manager = Arc::new(
            P2PManager::new(&config.p2p, storage.clone())
                .await
                .context("failed to initialize p2p manager")?,
        );

        let sync_manager = Arc::new(
            SyncManager::new(
                &config.sequencer,
                &config.l1,
                storage.clone(),
                verification_manager.clone(),
                p2p_manager.clone(),
            )
            .await
            .context("failed to initialize sync manager")?,
        );

        let rpc_server = if config.rpc.enabled {
            Some(Arc::new(
                RpcServer::new(
                    &config.rpc,
                    &config.websocket,
                    storage.clone(),
                    sync_manager.clone(),
                )
                .await
                .context("failed to initialize RPC server")?,
            ))
        } else {
            None
        };

        let metrics_server = if config.metrics.enabled {
            Some(Arc::new(
                MetricsServer::new(&config.metrics)
                    .await
                    .context("failed to initialize metrics server")?,
            ))
        } else {
            None
        };

        Ok(Self {
            config,
            storage,
            sync_manager,
            rpc_server,
            p2p_manager,
            metrics_server,
            shutdown_tx,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting node services");

        let mut tasks = Vec::new();

        if self.config.p2p.enabled {
            let manager = self.p2p_manager.clone();
            let mut shutdown = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    res = manager.run() => if let Err(e) = res { error!("P2P manager error: {}", e); },
                    _ = shutdown.recv() => info!("P2P manager shutting down"),
                }
            }));
        }

        let sync_manager = self.sync_manager.clone();
        let mut sync_shutdown = self.shutdown_tx.subscribe();
        tasks.push(tokio::spawn(async move {
            tokio::select! {
                res = sync_manager.run() => if let Err(e) = res { error!("Sync manager error: {}", e); },
                _ = sync_shutdown.recv() => info!("Sync manager shutting down"),
            }
        }));

        if let Some(server) = &self.rpc_server {
            let server = Arc::clone(server);
            let mut shutdown = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    res = server.run() => if let Err(e) = res { error!("RPC server error: {}", e); },
                    _ = shutdown.recv() => info!("RPC server shutting down"),
                }
            }));
        }

        if let Some(metrics) = &self.metrics_server {
            let metrics = Arc::clone(metrics);
            let mut shutdown = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    res = metrics.run() => if let Err(e) = res { error!("Metrics server error: {}", e); },
                    _ = shutdown.recv() => info!("Metrics server shutting down"),
                }
            }));
        }

        let storage = self.storage.clone();
        let sync_manager = self.sync_manager.clone();
        let mut health_shutdown = self.shutdown_tx.subscribe();
        tasks.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::health_check(&storage, &sync_manager).await;
                    }
                    _ = health_shutdown.recv() => {
                        info!("Health check task shutting down");
                        break;
                    }
                }
            }
        }));

        let mut shutdown = self.shutdown_tx.subscribe();
        shutdown.recv().await.ok();

        join_all(tasks).await;
        info!("All services stopped");
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating shutdown");
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!("Failed to broadcast shutdown: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.storage.close().await?;
        info!("Shutdown complete");
        Ok(())
    }

    async fn health_check(storage: &Storage, sync_manager: &SyncManager) {
        debug!("Performing health check");
        if let Err(e) = storage.health_check().await {
            warn!("Storage health check failed: {}", e);
        }
        let status = sync_manager.get_status().await;
        if status.highest_block > status.current_block + 100 {
            warn!(
                "Node is behind by {} blocks",
                status.highest_block - status.current_block
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_node_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.rpc.enabled = false;
        config.websocket.enabled = false;
        config.metrics.enabled = false;
        config.p2p.discovery_enabled = false;

        Node::new(config).await.unwrap();
    }
}
