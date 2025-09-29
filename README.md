# Enter L2 Node

Public node software for the Enter L2 network that allows anyone to run a full node, verify the network, and provide RPC services.

## Overview

The Enter L2 Node is the public interface to the Enter L2 network, providing:

- **Full Node Operation**: Sync with the network and maintain complete L2 state
- **Independent Verification**: Verify ZK proofs and state transitions
- **JSON-RPC API**: Ethereum-compatible API for wallet and dApp integration
- **WebSocket Support**: Real-time event subscriptions
- **Data Availability**: Serve historical data and state queries
- **P2P Networking**: Participate in the decentralized network

## Features

### 🔗 Ethereum Compatibility
- Full JSON-RPC API compatibility with Ethereum tooling
- Support for MetaMask, Hardhat, Foundry, and other tools
- Standard transaction and block formats
- Event logs and filtering

### 🔍 Independent Verification
- Verify ZK proofs for all batches
- Validate state transitions independently
- Detect and report fraud or inconsistencies
- Monitor network liveness and censorship

### 📊 Data Services
- Complete transaction history
- Account state queries
- Block and batch data
- Bridge operation tracking
- Staking and naming information

### 🌐 Network Participation
- P2P networking with other nodes
- Transaction relay and gossip
- Block synchronization
- Peer discovery and management

## Installation

### From Binary

Download the latest release from [GitHub Releases](https://github.com/enter-l2/node/releases):

```bash
# Linux
wget https://github.com/enter-l2/node/releases/latest/download/enter-l2-node-linux-amd64.tar.gz
tar -xzf enter-l2-node-linux-amd64.tar.gz
sudo mv enter-l2-node /usr/local/bin/

# macOS
wget https://github.com/enter-l2/node/releases/latest/download/enter-l2-node-darwin-amd64.tar.gz
tar -xzf enter-l2-node-darwin-amd64.tar.gz
sudo mv enter-l2-node /usr/local/bin/

# Windows
# Download enter-l2-node-windows-amd64.zip and extract
```

### From Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/enter-l2/node.git
cd node
cargo build --release

# Install binary
sudo cp target/release/enter-l2-node /usr/local/bin/
```

### Docker

```bash
# Run with Docker
docker run -d \
  --name enter-l2-node \
  -p 8545:8545 \
  -p 8546:8546 \
  -v ./data:/data \
  enterl2/node:latest

# Or use Docker Compose
curl -O https://raw.githubusercontent.com/enter-l2/node/main/docker-compose.yml
docker-compose up -d
```

## Quick Start

### Basic Usage

```bash
# Start a full node
enter-l2-node --data-dir ./data

# Start with custom RPC port
enter-l2-node --rpc-port 8545 --ws-port 8546

# Start in light mode
enter-l2-node --mode light

# Start with specific sequencer
enter-l2-node --sequencer-url https://sequencer.enterl2.com
```

### Configuration File

Create `config.toml`:

```toml
[node]
mode = "full"
data_dir = "./data"

[rpc]
enabled = true
port = 8545
host = "0.0.0.0"

[websocket]
enabled = true
port = 8546
host = "0.0.0.0"

[sequencer]
url = "https://sequencer.enterl2.com"
timeout = "30s"

[l1]
rpc_url = "https://mainnet.infura.io/v3/YOUR_INFURA_KEY"
contracts = { state_manager = "0x...", bridge = "0x..." }

[p2p]
enabled = true
port = 30303
max_peers = 50
bootstrap_nodes = [
  "/ip4/1.2.3.4/tcp/30303/p2p/12D3KooW...",
  "/ip4/5.6.7.8/tcp/30303/p2p/12D3KooW..."
]

[database]
url = "postgres://user:pass@localhost/enterl2"

[metrics]
enabled = true
port = 9090
```

Then run:

```bash
enter-l2-node --config config.toml
```

## Node Modes

### Full Node
```bash
enter-l2-node --mode full
```
- Complete transaction history
- Full state verification
- Serves all RPC methods
- Participates in P2P network

### Light Node
```bash
enter-l2-node --mode light
```
- Minimal storage requirements
- Trusts sequencer for most data
- Basic RPC functionality
- Suitable for wallets and light clients

### Archive Node
```bash
enter-l2-node --mode archive
```
- Complete historical data
- All state transitions
- Enhanced indexing
- Suitable for block explorers

### RPC Node
```bash
enter-l2-node --mode rpc --max-connections 1000
```
- Optimized for serving requests
- High connection limits
- Enhanced caching
- Suitable for public RPC services

## API Reference

### Ethereum-Compatible Methods

```bash
# Get latest block number
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  http://localhost:8545

# Get account balance
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0x742d35Cc6634C0532925a3b8D4C9db96c4b4d8b","latest"],"id":1}' \
  http://localhost:8545

# Send transaction
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0x..."],"id":1}' \
  http://localhost:8545
```

### Enter L2 Specific Methods

```bash
# Get batch information
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getBatchByNumber","params":["0x1"],"id":1}' \
  http://localhost:8545

# Get staking information
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getStakingInfo","params":["0x742d35Cc6634C0532925a3b8D4C9db96c4b4d8b"],"id":1}' \
  http://localhost:8545

# Get name information
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getNameInfo","params":["alice"],"id":1}' \
  http://localhost:8545

# Get node status
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getNodeStatus","params":[],"id":1}' \
  http://localhost:8545
```

### WebSocket Events

```javascript
const ws = new WebSocket('ws://localhost:8546');

ws.onopen = function() {
  // Subscribe to new blocks
  ws.send(JSON.stringify({
    jsonrpc: '2.0',
    method: 'eth_subscribe',
    params: ['newHeads'],
    id: 1
  }));
  
  // Subscribe to pending transactions
  ws.send(JSON.stringify({
    jsonrpc: '2.0',
    method: 'eth_subscribe',
    params: ['newPendingTransactions'],
    id: 2
  }));
};

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

## Integration Examples

### MetaMask

Add Enter L2 network to MetaMask:

```javascript
await window.ethereum.request({
  method: 'wallet_addEthereumChain',
  params: [{
    chainId: '0xa4b1',
    chainName: 'Enter L2',
    nativeCurrency: {
      name: 'Ether',
      symbol: 'ETH',
      decimals: 18
    },
    rpcUrls: ['http://localhost:8545'],
    blockExplorerUrls: ['https://explorer.enterl2.com']
  }]
});
```

### Hardhat

Configure Hardhat to use your node:

```javascript
// hardhat.config.js
module.exports = {
  networks: {
    enterl2: {
      url: "http://localhost:8545",
      accounts: ["0x..."] // Your private keys
    }
  }
};
```

### Foundry

```bash
# Deploy contract
forge create --rpc-url http://localhost:8545 \
  --private-key 0x... \
  src/MyContract.sol:MyContract

# Call contract
cast call --rpc-url http://localhost:8545 \
  0x... "balanceOf(address)" 0x...
```

### Web3.js

```javascript
const Web3 = require('web3');
const web3 = new Web3('http://localhost:8545');

// Get latest block
const block = await web3.eth.getBlock('latest');
console.log('Latest block:', block.number);

// Send transaction
const tx = await web3.eth.sendSignedTransaction('0x...');
console.log('Transaction hash:', tx.transactionHash);
```

### Ethers.js

```javascript
const { ethers } = require('ethers');
const provider = new ethers.JsonRpcProvider('http://localhost:8545');

// Get balance
const balance = await provider.getBalance('0x...');
console.log('Balance:', ethers.formatEther(balance));

// Send transaction
const wallet = new ethers.Wallet('0x...', provider);
const tx = await wallet.sendTransaction({
  to: '0x...',
  value: ethers.parseEther('1.0')
});
```

## Monitoring

### Metrics

Access Prometheus metrics at `http://localhost:9090/metrics`:

```
# Node status
enterl2_node_syncing{} 0
enterl2_node_current_block{} 12345
enterl2_node_peer_count{} 25

# RPC metrics
enterl2_rpc_requests_total{method="eth_getBalance"} 1000
enterl2_rpc_request_duration_seconds{method="eth_getBalance"} 0.05

# Storage metrics
enterl2_storage_size_bytes{} 1073741824
enterl2_storage_operations_total{operation="read"} 50000
```

### Health Check

```bash
# Check node health
curl http://localhost:8545 \
  -X POST \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getNodeStatus","params":[],"id":1}'
```

### Logs

```bash
# View logs
journalctl -u enter-l2-node -f

# Or with Docker
docker logs -f enter-l2-node
```

## Performance Tuning

### Hardware Requirements

**Minimum (Light Node):**
- 2 CPU cores
- 4 GB RAM
- 50 GB SSD storage
- 10 Mbps internet

**Recommended (Full Node):**
- 4 CPU cores
- 8 GB RAM
- 500 GB SSD storage
- 100 Mbps internet

**High Performance (RPC Node):**
- 8+ CPU cores
- 16+ GB RAM
- 1+ TB NVMe SSD
- 1+ Gbps internet

### Configuration Tuning

```toml
[database]
# Use faster database settings
max_connections = 100
connection_timeout = "30s"
query_timeout = "10s"

[rpc]
# Increase connection limits
max_connections = 1000
request_timeout = "30s"
batch_limit = 100

[p2p]
# Optimize networking
max_peers = 100
connection_timeout = "10s"
keep_alive_interval = "30s"

[sync]
# Faster synchronization
batch_size = 1000
concurrent_requests = 10
verification_workers = 4
```

## Troubleshooting

### Common Issues

#### Sync Problems
```bash
# Check sync status
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"enterl2_getNodeStatus","params":[],"id":1}' \
  http://localhost:8545

# Restart sync from specific block
enter-l2-node --reset-to-block 12345
```

#### Connection Issues
```bash
# Check network connectivity
enter-l2-node --check-connectivity

# Test RPC endpoint
curl -X POST http://localhost:8545 \
  --data '{"jsonrpc":"2.0","method":"net_version","params":[],"id":1}'
```

#### Storage Issues
```bash
# Check disk space
df -h

# Compact database
enter-l2-node --compact-db

# Reset data directory
rm -rf ./data && enter-l2-node --data-dir ./data
```

### Debug Mode

```bash
# Run with debug logging
enter-l2-node --log-level debug

# Enable specific debug modules
RUST_LOG=enter_l2_node::sync=debug,enter_l2_node::rpc=info enter-l2-node
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Run benchmarks: `cargo bench`
6. Submit a pull request

## Security

### Reporting Vulnerabilities

Report security issues to security@enterl2.com

### Security Considerations

- Keep your node updated
- Use firewall rules to restrict access
- Monitor for unusual activity
- Backup your data directory regularly

## License

MIT License - see LICENSE file for details.

## Support

- **Documentation**: [docs.enterl2.com](https://docs.enterl2.com)
- **Discord**: [discord.gg/enterl2](https://discord.gg/enterl2)
- **GitHub Issues**: [github.com/enter-l2/node/issues](https://github.com/enter-l2/node/issues)
- **Email**: support@enterl2.com
