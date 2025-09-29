//! Enter L2 Node
//! 
//! Public node software for the Enter L2 network that provides:
//! - Full node synchronization with the network
//! - JSON-RPC API compatible with Ethereum tooling
//! - WebSocket support for real-time events
//! - Independent verification of ZK proofs
//! - Data availability and historical queries

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, warn, error};

mod config;
mod node;
mod rpc;
mod sync;
mod storage;
mod p2p;
mod verification;
mod metrics;
mod cli;

use config::NodeConfig;
use node::Node;

#[derive(Parser)]
#[command(name = "enter-l2-node")]
#[command(about = "Enter L2 Node - Public node software for Enter L2 network")]
#[command(version = "1.0.0")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    
    /// Data directory
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Node mode
    #[arg(short, long, default_value = "full")]
    mode: String,
    
    /// Enable RPC server
    #[arg(long, default_value = "true")]
    enable_rpc: bool,
    
    /// RPC port
    #[arg(long, default_value = "8545")]
    rpc_port: u16,
    
    /// Enable WebSocket server
    #[arg(long, default_value = "true")]
    enable_ws: bool,
    
    /// WebSocket port
    #[arg(long, default_value = "8546")]
    ws_port: u16,
    
    /// Enable metrics server
    #[arg(long, default_value = "true")]
    enable_metrics: bool,
    
    /// Metrics port
    #[arg(long, default_value = "9090")]
    metrics_port: u16,
    
    /// Sequencer URL
    #[arg(long, default_value = "https://sequencer.enterl2.com")]
    sequencer_url: String,
    
    /// L1 RPC URL
    #[arg(long)]
    l1_rpc_url: Option<String>,
    
    /// Maximum peers
    #[arg(long, default_value = "50")]
    max_peers: usize,
    
    /// Disable peer discovery
    #[arg(long)]
    no_discovery: bool,
    
    /// Bootstrap nodes
    #[arg(long)]
    bootstrap_nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(&args.log_level)?;
    
    info!("Starting Enter L2 Node v{}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {}", args.data_dir.display());
    info!("Node mode: {}", args.mode);
    
    // Load configuration
    let mut config = if args.config.exists() {
        NodeConfig::from_file(&args.config)?
    } else {
        warn!("Configuration file not found, using defaults");
        NodeConfig::default()
    };
    
    // Override config with CLI arguments
    config.data_dir = args.data_dir;
    config.node.mode = args.mode;
    config.rpc.enabled = args.enable_rpc;
    config.rpc.port = args.rpc_port;
    config.websocket.enabled = args.enable_ws;
    config.websocket.port = args.ws_port;
    config.metrics.enabled = args.enable_metrics;
    config.metrics.port = args.metrics_port;
    config.sequencer.url = args.sequencer_url;
    config.p2p.max_peers = args.max_peers;
    config.p2p.discovery_enabled = !args.no_discovery;
    config.p2p.bootstrap_nodes = args.bootstrap_nodes;
    
    if let Some(l1_url) = args.l1_rpc_url {
        config.l1.rpc_url = l1_url;
    }
    
    // Validate configuration
    config.validate()?;
    
    // Create and start node
    let node = Node::new(config).await?;
    
    // Handle shutdown signals
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received shutdown signal");
    };
    
    // Run node until shutdown
    tokio::select! {
        result = node.run() => {
            match result {
                Ok(_) => info!("Node stopped gracefully"),
                Err(e) => error!("Node error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("Shutting down node...");
        }
    }
    
    // Graceful shutdown
    node.shutdown().await?;
    info!("Node shutdown complete");
    
    Ok(())
}

fn init_logging(level: &str) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(env_filter)
        .init();
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_node_startup() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.rpc.enabled = false;
        config.websocket.enabled = false;
        config.metrics.enabled = false;
        config.p2p.discovery_enabled = false;
        
        let node = Node::new(config).await.unwrap();
        
        // Test that node can be created without errors
        assert!(node.is_initialized());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = NodeConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid data directory should fail
        config.data_dir = PathBuf::from("/invalid/path/that/does/not/exist");
        assert!(config.validate().is_err());
    }
}
