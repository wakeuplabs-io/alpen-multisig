# regtest-dev-api

Small HTTP server for regtest development actions.

## Run

```bash
cargo run -p regtest-dev-api
```

Starts on port `3001` by default.

## Endpoints

**Mine blocks**
```bash
curl -X POST "http://localhost:3001/mine?count=5"
```

**Faucet**
```bash
curl -X POST "http://localhost:3001/faucet" \
  -H "Content-Type: application/json" \
  -d '{"address":"bcrt1q...","amount_btc":1.0}'
```

## Config (env vars)

| Variable | Default |
|----------|---------|
| `BITCOIN_RPC_URL` | `http://127.0.0.1:18443` |
| `BITCOIN_RPC_USER` | `user` |
| `BITCOIN_RPC_PASS` | `password` |
| `BITCOIN_RPC_WALLET` | first loaded wallet |
| `SERVER_PORT` | `3001` |
