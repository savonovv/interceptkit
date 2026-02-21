# Contributing

Thanks for helping improve InterceptKit.

## Development Setup

1. Proxy:

```bash
cargo run --manifest-path proxy/Cargo.toml
```

2. Extension:

```bash
cd extension
npm install
npm run build
```

Load unpacked extension from `extension/dist/firefox` (Firefox/Zen) or `extension/dist/chrome` (Chrome).

## Pull Request Guidelines

- Keep changes focused and small.
- Update docs when behavior changes.
- Run checks before opening PR:
  - `cargo check --manifest-path proxy/Cargo.toml`
  - `npm run typecheck && npm run build` in `extension/`

## Security and Privacy

- Do not log secrets in plaintext.
- Minimize new extension permissions.
- Do not introduce remote code execution paths.
