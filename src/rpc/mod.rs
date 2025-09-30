//! JSON-RPC server implementation
//! 
//! Provides Ethereum-compatible JSON-RPC API for Enter L2 network

use anyhow::Result;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::config::{RpcConfig, WebSocketConfig};
use crate::storage::Storage;
use crate::sync::SyncManager;

mod methods;
mod websocket;

use methods::RpcMethods;
use websocket::WebSocketServer;

/// JSON-RPC server
pub struct RpcServer {
    config: RpcConfig,
    methods: Arc<RpcMethods>,
    ws_server: Option<Arc<WebSocketServer>>,
}

impl RpcServer {
    /// Create a new RPC server
    pub async fn new(
        rpc_config: &RpcConfig,
        ws_config: &WebSocketConfig,
        storage: Arc<Storage>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Self> {
        let methods = Arc::new(RpcMethods::new(storage, sync_manager));
        
        let ws_server = if ws_config.enabled {
            Some(Arc::new(WebSocketServer::new(ws_config, methods.clone()).await?))
        } else {
            None
        };
        
        Ok(Self {
            config: rpc_config.clone(),
            methods,
            ws_server,
        })
    }
    
    /// Run the RPC server
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut tasks = Vec::new();

        // Start HTTP RPC server
        if self.config.enabled {
            let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
            let methods = self.methods.clone();
            
            let make_svc = make_service_fn(move |_conn| {
                let methods = methods.clone();
                async move {
                    Ok::<_, Infallible>(service_fn(move |req| {
                        let methods = methods.clone();
                        async move {
                            handle_request(req, methods).await
                        }
                    }))
                }
            });
            
            let server = Server::bind(&addr).serve(make_svc);
            info!("RPC server listening on {}", addr);
            
            tasks.push(tokio::spawn(async move {
                if let Err(e) = server.await {
                    error!("RPC server error: {}", e);
                }
            }));
        }
        
        // Start WebSocket server
        if let Some(ws_server) = self.ws_server.clone() {
            tasks.push(tokio::spawn(async move {
                if let Err(e) = ws_server.run().await {
                    error!("WebSocket server error: {}", e);
                }
            }));
        }
        
        // Wait for all tasks
        futures::future::join_all(tasks).await;
        
        Ok(())
    }
}

/// Handle HTTP request
async fn handle_request(
    req: Request<Body>,
    methods: Arc<RpcMethods>,
) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::POST => handle_rpc_request(req, methods).await,
        &Method::OPTIONS => handle_cors_preflight().await,
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::from("Method not allowed"))
            .unwrap()),
    }
}

/// Handle JSON-RPC request
async fn handle_rpc_request(
    req: Request<Body>,
    methods: Arc<RpcMethods>,
) -> Result<Response<Body>, Infallible> {
    // Add CORS headers
    let response = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Content-Type", "application/json");
    
    // Parse request body
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Ok(response
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                }).to_string()))
                .unwrap());
        }
    };
    
    // Parse JSON
    let request: Value = match serde_json::from_slice(&body_bytes) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to parse JSON: {}", e);
            return Ok(response
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                }).to_string()))
                .unwrap());
        }
    };
    
    // Handle batch or single request
    let response_json = if request.is_array() {
        handle_batch_request(request, methods).await
    } else {
        handle_single_request(request, methods).await
    };
    
    Ok(response
        .status(StatusCode::OK)
        .body(Body::from(response_json.to_string()))
        .unwrap())
}

/// Handle CORS preflight request
async fn handle_cors_preflight() -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .body(Body::empty())
        .unwrap())
}

/// Handle batch JSON-RPC request
async fn handle_batch_request(request: Value, methods: Arc<RpcMethods>) -> Value {
    let requests = match request.as_array() {
        Some(arr) => arr,
        None => return json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": null
        }),
    };
    
    if requests.is_empty() {
        return json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": null
        });
    }
    
    let mut responses = Vec::new();
    for req in requests {
        let response = handle_single_request(req.clone(), methods.clone()).await;
        responses.push(response);
    }
    
    Value::Array(responses)
}

/// Handle single JSON-RPC request
async fn handle_single_request(request: Value, methods: Arc<RpcMethods>) -> Value {
    // Validate JSON-RPC format
    let jsonrpc = request.get("jsonrpc").and_then(|v| v.as_str());
    let method = request.get("method").and_then(|v| v.as_str());
    let params = request.get("params");
    let id = request.get("id");
    
    if jsonrpc != Some("2.0") {
        return json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": id
        });
    }
    
    let method = match method {
        Some(m) => m,
        None => return json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": id
        }),
    };
    
    debug!("RPC call: {}", method);
    
    // Call method
    let result = methods.call(method, params).await;
    
    match result {
        Ok(value) => json!({
            "jsonrpc": "2.0",
            "result": value,
            "id": id
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "error": error,
            "id": id
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_rpc_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::new(temp_dir.path(), &Default::default()).await.unwrap());
        let sync_manager = Arc::new(SyncManager::new(
            &Default::default(),
            &Default::default(),
            storage.clone(),
            Arc::new(crate::verification::VerificationManager::new(&Default::default()).await.unwrap()),
            Arc::new(crate::p2p::P2PManager::new(&Default::default(), storage.clone()).await.unwrap()),
        ).await.unwrap());
        
        let rpc_config = RpcConfig {
            enabled: true,
            port: 8545,
            ..Default::default()
        };
        
        let ws_config = WebSocketConfig {
            enabled: false,
            ..Default::default()
        };
        
        let server = RpcServer::new(&rpc_config, &ws_config, storage, sync_manager).await.unwrap();
        // Test passes if server creation succeeds
    }
}
