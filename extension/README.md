# Extension

MV3 TypeScript extension for setup checks, rule management, and DevTools capture.

## Build

```bash
npm install
npm run build
```

Output folders:

- `dist/chrome`
- `dist/firefox`

## Load Unpacked

### Chrome

1. Open `chrome://extensions`.
2. Enable Developer Mode.
3. Click `Load unpacked` and choose `extension/dist/chrome`.

### Firefox

1. Open `about:debugging#/runtime/this-firefox`.
2. Click `Load Temporary Add-on`.
3. Choose `extension/dist/firefox/manifest.json`.
