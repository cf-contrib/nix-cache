# nix-cache

[![CI](https://github.com/cf-contrib/nix-cache/actions/workflows/ci.yml/badge.svg)](https://github.com/cf-contrib/nix-cache/actions/workflows/ci.yml)
[![Rust (edition 2021)](https://img.shields.io/badge/Rust-2021-black?logo=rust)](https://www.rust-lang.org/)
[![Nix Flake](https://img.shields.io/badge/Nix-Flake-5277C3?logo=nixos&logoColor=white)](https://nixos.wiki/wiki/Flakes)
[![License: MIT](https://img.shields.io/github/license/cf-contrib/nix-cache)](LICENSE)

Cloudflare-native Nix binary cache backed by **Workers + R2**, implemented in **Rust**.

> **Stop babysitting a Nix cache server.** Deploy this Worker, point `nix.conf` at it, and get reproducible, globally-cached substitutes on Cloudflare's edge — no VMs, no daemons, no ops.

## What this provides

- A Nix-compatible cache API:
  - `GET /nix-cache-info`
  - `GET /<hash>.narinfo`
  - `GET /nar/<hash>.nar`
- Upload endpoints (authenticated):
  - `PUT /<hash>.narinfo` (narinfo text)
  - `PUT /nar/<hash>.nar` (nar bytes)
- Optional server-side signing of `.narinfo` when the uploader does not provide `Sig:` entries.

## Deploying

This repository is intended to be **deployed with Terraform** (Cloudflare Workers + R2).

> Note: `wrangler.toml` is used for **local testing** only. We will add a Terraform example later.

### Release artifacts

The GitHub Release assets produced by CI contain the Worker runtime bundle:

- `build/index.js`
- `build/index_bg.wasm`

These are the minimum required files, because `build/index.js` imports `./index_bg.wasm`.

## Configuration

### Worker environment variables

- `NIX_TOKEN` (required for uploads)
  - Used for HTTP Basic auth on `PUT` requests.
  - Expected credentials: username `x-auth-token`, password `${NIX_TOKEN}`.
- `NIX_SIGNING_SECRET` (optional)
  - When set, the Worker will sign uploaded `.narinfo` that don’t include `Sig:`.
  - Format: `<key-name>:<base64>` where `<base64>` decodes to 64 Ed25519 key bytes (as emitted by `nix key generate-secret`).

### Cloudflare resources

- R2 bucket bound as `NIX_BUCKET` (stores `.narinfo` and `.nar` objects).

## Local development / testing

The development shell provides the required tooling (see `flake.nix`). Typical workflow:

- Run tests: `nix develop -c cargo test`
- Build the Worker bundle locally (for testing): `nix develop -c worker-build --dev`

`wrangler.toml` is configured to point `wrangler` at the generated bundle in `build/`.

## Dependencies

Runtime dependencies (crates) include:

- `worker` / `worker-macros` (Cloudflare Workers Rust SDK)
- `narinfo` (parse/serialize `.narinfo`)
- `http-auth-basic` (Basic Auth parsing)
- `ed25519-dalek-v2`, `sha2`, `base64` (optional `.narinfo` signing + validation)

Tooling:

- **Nix** (recommended) for a consistent dev environment
- **worker-build** (from Cloudflare’s `workers-rs`) to produce the JS+WASM bundle
- **wrangler** for local testing
- **Terraform** for production deployment (Cloudflare provider)

## License

MIT.
