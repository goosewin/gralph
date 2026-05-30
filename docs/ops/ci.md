# CI

This repository uses Woodpecker CI on private Goosewin infrastructure.

GitHub Actions is intentionally disabled for this repository. Keep CI server
locations, webhook URLs, Tailscale names, private IPs, and runner details out of
tracked files. Store provider endpoints and credentials only in GitHub or
Woodpecker secrets/settings.

Package repositories must use Bun 1.3.14 and the two-week package-age gate in
`bunfig.toml`.
