//! Core node implementation

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::config::NodeConfig;
use crate::storage::Storage;
use crate::sync::SyncManager;
use crate::rpc::RpcServer;
use crate::p2p::P2PManager;
use crate::verification::VerificationManager;
use crate::metrics::MetricsServer;

/// Main node structure that coordinates all components
pub struct Node {
    config: NodeConfig,
    storage: Arc<Storage>,
    sync_manager: Arc<SyncManager>,
    rpc_server: Option<RpcServer>,
    p2p_manager: Arc<P2PManager>,
    verification_manager: Arc<VerificationManager>,
    metrics_server: Option<MetricsServer>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl Node {
    /// Create a new node instance
    pub async fn new(config: NodeConfig) -> Result<Self> {
        info!("Initializing Enter L2 Node");
        
        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        
        // Initialize storage
        info!("Initializing storage...");
        let storage = Arc::new(
            Storage::new(&config.data_dir, &config.database)
                .await
                .context("Failed to initialize storage")?
        );
        
        // Initialize verification manager
        info!("Initializing verification manager...");
        let verification_manager = Arc::new(
            VerificationManager::new(&config.verification)
                .await
                .context("Failed to initialize verification manager")?
        );
        
        // Initialize P2P manager
        info!("Initializing P2P manager...");
        let p2p_manager = Arc::new(
            P2PManager::new(&config.p2p, storage.clone())
                .await
                .context("Failed to initialize P2P manager")?
        );
        
        // Initialize sync manager
        info!("Initializing sync manager...");
        let sync_manager = Arc::new(
            SyncManager::new(
                &config.sequencer,
                &config.l1,
                storage.clone(),
                verification_manager.clone(),
                p2p_manager.clone(),
            )
            .await
            .context("Failed to initialize sync manager")?
        );
        
        // Initialize RPC server if enabled
        let rpc_server = if config.rpc.enabled {
            info!("Initializing RPC server...");
            Some(
                RpcServer::new(
                    &config.rpc,
                    &config.websocket,
                    storage.clone(),
                    sync_manager.clone(),
                )
                .await
                .context("Failed to initialize RPC server")?
            )
        } else {
            None
        };
        
        // Initialize metrics server if enabled
        let metrics_server = if config.metrics.enabled {
            info!("Initializing metrics server...");
            Some(
                MetricsServer::new(&config.metrics)
                    .await
                    .context("Failed to initialize metrics server")?
            )
        } else {
            None
        };
        
        info!("Node initialization complete");
        
        Ok(Self {
            config,
            storage,
            sync_manager,
            rpc_server,
            p2p_manager,
            verification_manager,
            metrics_server,
            shutdown_tx,
            shutdown_rx,
        })
    }
    
    /// Run the node
    pub async fn run(self) -> Result<()> {
        info!("Starting Enter L2 Node services");
        
        let mut tasks = Vec::new();
        
        // Start P2P manager
        if self.config.p2p.enabled {
            info!("Starting P2P manager...");
            let p2p_manager = self.p2p_manager.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    result = p2p_manager.run() => {
                        if let Err(e) = result {
                            error!("P2P manager error: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("P2P manager received shutdown signal");
                    }
                }
            }));
        }
        
        // Start sync manager
        info!("Starting sync manager...");
        let sync_manager = self.sync_manager.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tasks.push(tokio::spawn(async move {
            tokio::select! {
                result = sync_manager.run() => {
                    if let Err(e) = result {
                        error!("Sync manager error: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Sync manager received shutdown signal");
                }
            }
        }));
        
        // Start RPC server
        if let Some(rpc_server) = self.rpc_server {
            info!("Starting RPC server on port {}...", self.config.rpc.port);
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    result = rpc_server.run() => {
                        if let Err(e) = result {
                            error!("RPC server error: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("RPC server received shutdown signal");
                    }
                }
            }));
        }
        
        // Start metrics server
        if let Some(metrics_server) = self.metrics_server {
            info!("Starting metrics server on port {}...", self.config.metrics.port);
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    result = metrics_server.run() => {
                        if let Err(e) = result {
                            error!("Metrics server error: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Metrics server received shutdown signal");
                    }
                }
            }));
        }
        
        // Start health check task
        let storage = self.storage.clone();
        let sync_manager = self.sync_manager.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tasks.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::health_check(&storage, &sync_manager).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Health check task received shutdown signal");
                        break;
                    }
                }
            }
        }));
        
        info!("All services started successfully");
        
        // Wait for all tasks to complete
        futures::future::join_all(tasks).await;
        
        info!("All services stopped");
        Ok(())
    }
    
    /// Shutdown the node gracefully
    pub async fn shutdown(self) -> Result<()> {
        info!("Initiating graceful shutdown...");
        
        // Send shutdown signal to all components
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }
        
        // Give components time to shutdown gracefully
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        // Close storage
        self.storage.close().await?;
        
        info!("Graceful shutdown complete");
        Ok(())
    }
    
    /// Check if node is initialized
    pub fn is_initialized(&self) -> bool {
        true // If we got here, initialization was successful
    }
    
    /// Get node status
    pub async fn get_status(&self) -> NodeStatus {
        let sync_status = self.sync_manager.get_status().await;
        let storage_status = self.storage.get_status().await;
        let p2p_status = self.p2p_manager.get_status().await;
        
        NodeStatus {
            is_syncing: sync_status.is_syncing,
            current_block: sync_status.current_block,
            highest_block: sync_status.highest_block,
            peer_count: p2p_status.peer_count,
            storage_size: storage_status.size_bytes,
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// Perform health check
    async fn health_check(storage: &Storage, sync_manager: &SyncManager) {
        debug!("Performing health check...");
        
        // Check storage health
        if let Err(e) = storage.health_check().await {
            warn!("Storage health check failed: {}", e);
        }
        
        // Check sync status
        let sync_status = sync_manager.get_status().await;
        if sync_status.is_syncing {
            info!(
                "Syncing: block {}/{}",
                sync_status.current_block,
                sync_status.highest_block
            );
        }
        
        // Check if we're falling behind
        if sync_status.highest_block > sync_status.current_block + 100 {
            warn!(
                "Node is falling behind: {} blocks behind",
                sync_status.highest_block - sync_status.current_block
            );
        }
    }
}

/// Node status information
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub is_syncing: bool,
    pub current_block: u64,
    pub highest_block: u64,
    pub peer_count: usize,
    pub storage_size: u64,
    pub uptime: u64,
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
        config.p2p.enabled = false;
        
        let node = Node::new(config).await.unwrap();
        assert!(node.is_initialized());
    }
    
    #[tokio::test]
    async fn test_node_status() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.rpc.enabled = false;
        config.websocket.enabled = false;
        config.metrics.enabled = false;
        config.p2p.enabled = false;
        
        let node = Node::new(config).await.unwrap();
        let status = node.get_status().await;
        
        assert_eq!(status.current_block, 0);
        assert_eq!(status.peer_count, 0);
    }
}
