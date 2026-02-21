# InterceptKit

InterceptKit is a local-first traffic interception toolkit made of:

- a browser extension (Chrome/Firefox) for setup, request capture, and rule authoring
- a local proxy daemon for request/response rewriting

This repository is a monorepo so extension, proxy, and shared contracts evolve together.

## Repository Layout

- `extension/` - MV3 TypeScript extension
- `proxy/` - Rust local proxy + control API
- `shared/` - shared schema and protocol notes
- `docs/` - setup and release playbooks

## MVP Capabilities

- setup checks from the extension (`proxy reachable`, `proxy configured`, `version compatible`)
- rule management (`create`, `list`, `delete`, `import draft from DevTools`)
- deterministic overlap resolution (`priority -> specificity -> rule id`)
- action modes: `mockResponse`, `rewritePassThrough`, and `sequence`

## Quick Start (Development)

1. Run proxy:

```bash
cargo run --manifest-path proxy/Cargo.toml
```

2. Build extension:

```bash
cd extension
npm install
npm run build
```

3. Load unpacked extension:
   - Firefox/Zen: `extension/dist/firefox`
   - Chrome: `extension/dist/chrome`

4. Open extension options page and run setup checks.

Detailed setup flow and distribution steps are in `docs/setup-and-release.md`.

## Public Release

- Tag a release (example: `v0.1.0`) and push the tag.
- GitHub Actions builds:
  - `interceptkit-proxy-<target>.tar.gz|zip`
  - `interceptkit-extension-firefox.zip`
  - `interceptkit-extension-chrome.zip`

Release workflow file: `.github/workflows/release.yml`.

## Notes

- HTTPS body rewriting requires local CA trust and MITM mode; this scaffold includes cert-status plumbing in the control API and setup UI.
- Current proxy implementation includes HTTP interception and CONNECT tunneling baseline for future full MITM expansion.
