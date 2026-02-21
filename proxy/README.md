# Proxy Service

Local Rust proxy plus control API for InterceptKit.

## Ports

- HTTP proxy listener: `127.0.0.1:8081`
- Control API: `127.0.0.1:4592`

Override with env vars:

- `INTERCEPTKIT_PROXY_PORT`
- `INTERCEPTKIT_CONTROL_PORT`

## Run

```bash
cargo run --manifest-path proxy/Cargo.toml
```

## Control API Endpoints

- `GET /health`
- `GET /version`
- `GET /status`
- `POST /status/interception`
- `POST /status/cert`
- `GET /rules`
- `POST /rules`
- `PUT /rules/:id`
- `DELETE /rules/:id`
- `GET /events`
- `DELETE /events`
- `POST /diagnostics/rewrite-check`
