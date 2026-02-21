# Setup and Release Flow

This document describes the user flow and recommended distribution strategy.

## User Setup Flow

1. Install InterceptKit extension (sideload unpacked build).
2. Open extension options page.
3. Click `Proxy Release` and download the proxy binary from GitHub Releases.
4. Run the proxy locally (default ports: proxy `8081`, control API `4592`).
5. Click `Enable Proxy` in extension options.
6. Click `Run Setup Check` and verify all checklist items.
7. Open DevTools panel `InterceptKit`, capture a request, and click `Create Rule`.
8. Trigger the same app request and verify rewritten/mock response behavior.

## What Setup Check Verifies

- `Proxy reachable` by calling `/health`.
- `Protocol compatible` by comparing extension protocol with `/version`.
- `Proxy configured` by checking browser proxy settings against local host/port.
- `Cert ready` and `MITM ready` from proxy status.
- `Diagnostics` by calling `/diagnostics/rewrite-check`.

## Current Runtime Capabilities

- deterministic rule resolution (`priority -> specificity -> rule id`)
- action modes: `mockResponse`, `rewritePassThrough`, and `sequence`
- request/response header/body transforms
- event logging and rule CRUD over local control API

## Current Networking Scope

- HTTP interception and rewriting are active.
- HTTPS requests are currently tunneled with CONNECT baseline.
- Full HTTPS body rewriting requires MITM cert installation and HTTPS interception mode expansion.

## Development Commands

### Proxy

```bash
cargo run --manifest-path proxy/Cargo.toml
```

### Extension

```bash
cd extension
npm install
npm run build
```

Then load unpacked extension from:

- `extension/dist/firefox` for Firefox/Zen
- `extension/dist/chrome` for Chrome

## Release Strategy (Recommended)

### GitHub Releases

Publish from one repository with two artifact families:

- `interceptkit-proxy-<rust-target>.tar.gz|zip` binaries
- `interceptkit-extension-chrome.zip` and `interceptkit-extension-firefox.zip`

### Extension Onboarding

- keep a stable `Proxy Release` link in options page
- include installer instructions in release notes
- expose setup checker output for self-service troubleshooting

### Suggested First Public Assets

- `quickstart.md` (60-second setup)
- 3 rule presets (`auth`, `crud`, `flaky endpoint`)
- short GIF: capture request -> create rule -> rewritten response
