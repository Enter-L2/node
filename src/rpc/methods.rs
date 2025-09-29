//! JSON-RPC method implementations

use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::storage::Storage;
use crate::sync::SyncManager;

/// RPC method implementations
pub struct RpcMethods {
    storage: Arc<Storage>,
    sync_manager: Arc<SyncManager>,
}

impl RpcMethods {
    /// Create new RPC methods handler
    pub fn new(storage: Arc<Storage>, sync_manager: Arc<SyncManager>) -> Self {
        Self {
            storage,
            sync_manager,
        }
    }
    
    /// Call an RPC method
    pub async fn call(&self, method: &str, params: Option<&Value>) -> Result<Value, Value> {
        match method {
            // Ethereum-compatible methods
            "eth_chainId" => self.eth_chain_id().await,
            "eth_blockNumber" => self.eth_block_number().await,
            "eth_getBalance" => self.eth_get_balance(params).await,
            "eth_getTransactionByHash" => self.eth_get_transaction_by_hash(params).await,
            "eth_getTransactionReceipt" => self.eth_get_transaction_receipt(params).await,
            "eth_getBlockByNumber" => self.eth_get_block_by_number(params).await,
            "eth_getBlockByHash" => self.eth_get_block_by_hash(params).await,
            "eth_sendRawTransaction" => self.eth_send_raw_transaction(params).await,
            "eth_call" => self.eth_call(params).await,
            "eth_estimateGas" => self.eth_estimate_gas(params).await,
            "eth_gasPrice" => self.eth_gas_price().await,
            "eth_getLogs" => self.eth_get_logs(params).await,
            "net_version" => self.net_version().await,
            "net_listening" => self.net_listening().await,
            "net_peerCount" => self.net_peer_count().await,
            "web3_clientVersion" => self.web3_client_version().await,
            
            // Enter L2 specific methods
            "enterl2_getBatchByNumber" => self.enterl2_get_batch_by_number(params).await,
            "enterl2_getBatchByHash" => self.enterl2_get_batch_by_hash(params).await,
            "enterl2_getStakingInfo" => self.enterl2_get_staking_info(params).await,
            "enterl2_getNameInfo" => self.enterl2_get_name_info(params).await,
            "enterl2_getBridgeStatus" => self.enterl2_get_bridge_status(params).await,
            "enterl2_getWalletInfo" => self.enterl2_get_wallet_info(params).await,
            "enterl2_getNodeStatus" => self.enterl2_get_node_status().await,
            
            _ => Err(json!({
                "code": -32601,
                "message": "Method not found"
            })),
        }
    }
    
    // Ethereum-compatible methods
    
    async fn eth_chain_id(&self) -> Result<Value, Value> {
        Ok(json!("0xa4b1")) // Enter L2 chain ID
    }
    
    async fn eth_block_number(&self) -> Result<Value, Value> {
        let block_number = self.storage.get_latest_block_number().await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get block number: {}", e)
            }))?;
        
        Ok(json!(format!("0x{:x}", block_number)))
    }
    
    async fn eth_get_balance(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let address = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid address parameter"
            }))?;
        
        let block_number = params.get(1)
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
        
        let balance = self.storage.get_balance(address, None, block_number).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get balance: {}", e)
            }))?;
        
        Ok(json!(format!("0x{:x}", balance)))
    }
    
    async fn eth_get_transaction_by_hash(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let tx_hash = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid transaction hash parameter"
            }))?;
        
        let transaction = self.storage.get_transaction_by_hash(tx_hash).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get transaction: {}", e)
            }))?;
        
        match transaction {
            Some(tx) => Ok(json!(tx)),
            None => Ok(Value::Null),
        }
    }
    
    async fn eth_get_transaction_receipt(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let tx_hash = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid transaction hash parameter"
            }))?;
        
        let receipt = self.storage.get_transaction_receipt(tx_hash).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get receipt: {}", e)
            }))?;
        
        match receipt {
            Some(receipt) => Ok(json!(receipt)),
            None => Ok(Value::Null),
        }
    }
    
    async fn eth_get_block_by_number(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let block_number = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid block number parameter"
            }))?;
        
        let include_txs = params.get(1)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let block = self.storage.get_block_by_number(block_number, include_txs).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get block: {}", e)
            }))?;
        
        match block {
            Some(block) => Ok(json!(block)),
            None => Ok(Value::Null),
        }
    }
    
    async fn eth_get_block_by_hash(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let block_hash = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid block hash parameter"
            }))?;
        
        let include_txs = params.get(1)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let block = self.storage.get_block_by_hash(block_hash, include_txs).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get block: {}", e)
            }))?;
        
        match block {
            Some(block) => Ok(json!(block)),
            None => Ok(Value::Null),
        }
    }
    
    async fn eth_send_raw_transaction(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let raw_tx = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid raw transaction parameter"
            }))?;
        
        let tx_hash = self.sync_manager.submit_transaction(raw_tx).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to submit transaction: {}", e)
            }))?;
        
        Ok(json!(tx_hash))
    }
    
    async fn eth_call(&self, _params: Option<&Value>) -> Result<Value, Value> {
        // Simplified implementation
        Ok(json!("0x"))
    }
    
    async fn eth_estimate_gas(&self, _params: Option<&Value>) -> Result<Value, Value> {
        // Return default gas estimate
        Ok(json!("0x5208")) // 21000 gas
    }
    
    async fn eth_gas_price(&self) -> Result<Value, Value> {
        let gas_price = self.storage.get_gas_price().await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get gas price: {}", e)
            }))?;
        
        Ok(json!(format!("0x{:x}", gas_price)))
    }
    
    async fn eth_get_logs(&self, _params: Option<&Value>) -> Result<Value, Value> {
        // Simplified implementation
        Ok(json!([]))
    }
    
    async fn net_version(&self) -> Result<Value, Value> {
        Ok(json!("42161")) // Enter L2 network ID
    }
    
    async fn net_listening(&self) -> Result<Value, Value> {
        Ok(json!(true))
    }
    
    async fn net_peer_count(&self) -> Result<Value, Value> {
        let peer_count = self.sync_manager.get_peer_count().await;
        Ok(json!(format!("0x{:x}", peer_count)))
    }
    
    async fn web3_client_version(&self) -> Result<Value, Value> {
        Ok(json!(format!("enter-l2-node/{}", env!("CARGO_PKG_VERSION"))))
    }
    
    // Enter L2 specific methods
    
    async fn enterl2_get_batch_by_number(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let batch_number = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid batch number parameter"
            }))?;
        
        let batch = self.storage.get_batch_by_number(batch_number).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get batch: {}", e)
            }))?;
        
        match batch {
            Some(batch) => Ok(json!(batch)),
            None => Ok(Value::Null),
        }
    }
    
    async fn enterl2_get_batch_by_hash(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let batch_hash = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid batch hash parameter"
            }))?;
        
        let batch = self.storage.get_batch_by_hash(batch_hash).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get batch: {}", e)
            }))?;
        
        match batch {
            Some(batch) => Ok(json!(batch)),
            None => Ok(Value::Null),
        }
    }
    
    async fn enterl2_get_staking_info(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let address = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid address parameter"
            }))?;
        
        let staking_info = self.storage.get_staking_info(address).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get staking info: {}", e)
            }))?;
        
        Ok(json!(staking_info))
    }
    
    async fn enterl2_get_name_info(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let name = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid name parameter"
            }))?;
        
        let name_info = self.storage.get_name_info(name).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get name info: {}", e)
            }))?;
        
        match name_info {
            Some(info) => Ok(json!(info)),
            None => Ok(Value::Null),
        }
    }
    
    async fn enterl2_get_bridge_status(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let tx_hash = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid transaction hash parameter"
            }))?;
        
        let bridge_status = self.storage.get_bridge_status(tx_hash).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get bridge status: {}", e)
            }))?;
        
        match bridge_status {
            Some(status) => Ok(json!(status)),
            None => Ok(Value::Null),
        }
    }
    
    async fn enterl2_get_wallet_info(&self, params: Option<&Value>) -> Result<Value, Value> {
        let params = params.ok_or_else(|| json!({
            "code": -32602,
            "message": "Invalid params"
        }))?;
        
        let address = params.get(0)
            .and_then(|v| v.as_str())
            .ok_or_else(|| json!({
                "code": -32602,
                "message": "Invalid address parameter"
            }))?;
        
        let wallet_info = self.storage.get_wallet_info(address).await
            .map_err(|e| json!({
                "code": -32000,
                "message": format!("Failed to get wallet info: {}", e)
            }))?;
        
        match wallet_info {
            Some(info) => Ok(json!(info)),
            None => Ok(Value::Null),
        }
    }
    
    async fn enterl2_get_node_status(&self) -> Result<Value, Value> {
        let sync_status = self.sync_manager.get_status().await;
        let storage_status = self.storage.get_status().await;
        
        Ok(json!({
            "syncing": sync_status.is_syncing,
            "currentBlock": sync_status.current_block,
            "highestBlock": sync_status.highest_block,
            "peerCount": sync_status.peer_count,
            "storageSize": storage_status.size_bytes,
            "version": env!("CARGO_PKG_VERSION")
        }))
    }
}
