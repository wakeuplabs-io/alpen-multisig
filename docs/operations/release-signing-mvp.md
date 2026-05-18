# Release signing MVP (P-011d)

Wave 2 delivers signed releases for **one** target OS as a starting point; Wave 3 completes Apple Authenticode, PGP manifests, and Tauri updater verification across all platforms (PRD NF-3).

## Linux MVP (this wave)

1. Build: `cd desktop-app && npm run tauri build -- --target x86_64-unknown-linux-gnu`
2. Sign the `.deb` or AppImage artifact with the project PGP release key (key id recorded in internal ops vault).
3. Publish detached signature: `*.deb.asc` or `*.AppImage.asc` alongside the binary in the GitHub Release.

## Verification

```bash
gpg --verify artifact.deb.asc artifact.deb
```

## Deferred (Wave 3)

- macOS notarization / Apple Developer ID
- Windows Authenticode
- Multi-employee signing ceremony per PRD NF-3
