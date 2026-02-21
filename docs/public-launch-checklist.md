# Public Launch Checklist (v0.1.0)

## Before Publishing

- [ ] Replace release URL in `extension/src/lib/constants.ts` if org/repo differs.
- [ ] Verify extension loads in both targets:
  - [ ] Firefox/Zen (`extension/dist/firefox`)
  - [ ] Chrome (`extension/dist/chrome`)
- [ ] Confirm proxy starts and `/health` returns `{ "ok": true }`.
- [ ] Confirm rule create + mock flow works with a public JSON endpoint.
- [ ] Review permissions and privacy wording in README.

## Create Release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `Release` workflow publishes:

- proxy binaries for Linux/macOS/Windows
- `interceptkit-extension-firefox.zip`
- `interceptkit-extension-chrome.zip`

## First Release Notes Template

- Local-first proxy + extension architecture
- Setup checker (proxy reachable / proxy configured / protocol compatibility)
- DevTools capture -> create rule
- Deterministic rule overlap resolution
- HTTP rewriting in MVP + HTTPS tunnel baseline
