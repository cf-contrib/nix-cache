# nix-cache

> **Stop babysitting a Nix cache server.** Deploy this Worker, point `nix.conf` at it, and get reproducible, globally-cached substitutes on Cloudflare's edge — no VMs, no daemons, no ops.

[![CI](https://github.com/cf-contrib/nix-cache/actions/workflows/ci.yml/badge.svg)](https://github.com/cf-contrib/nix-cache/actions/workflows/ci.yml)
[![Rust (edition 2021)](https://img.shields.io/badge/Rust-2021-black?logo=rust)](https://www.rust-lang.org/)
[![Nix Flake](https://img.shields.io/badge/Nix-Flake-5277C3?logo=nixos&logoColor=white)](https://nixos.wiki/wiki/Flakes)
[![License: MIT](https://img.shields.io/github/license/cf-contrib/nix-cache)](LICENSE)

A Cloudflare-native [Nix](https://nixos.org/) binary cache. Runs on **Workers + R2**, written in **Rust**.

## Table of contents

- [Features](#features)
- [How it works](#how-it-works)
- [Quick start](#quick-start)
- [HTTP API](#http-api)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Development](#development)
- [Dependencies](#dependencies)
- [License](#license)

## Features

- Drop-in replacement for `cache.nixos.org`-style binary caches.
- Globally distributed reads via Cloudflare's edge.
- Durable object storage backed by R2 (no egress fees to Workers).
- Authenticated uploads with HTTP Basic auth.
- Optional server-side Ed25519 signing of `.narinfo` files.
- Single Worker bundle (`index.js` + `index_bg.wasm`) — no runtime dependencies.

## How it works

```
┌──────────┐    GET /<hash>.narinfo    ┌──────────────┐    R2 GET    ┌────────┐
│   nix    │ ────────────────────────► │  Worker (CF) │ ───────────► │   R2   │
│  client  │ ◄──────────────────────── │              │ ◄─────────── │ bucket │
└──────────┘     narinfo + .nar        └──────────────┘    object    └────────┘
                                              ▲
                                              │ PUT (Basic auth)
                                       ┌──────────────┐
                                       │  uploader    │
                                       │ (nix copy …) │
                                       └──────────────┘
```

The Worker serves the Nix binary cache protocol from an R2 bucket. Uploads are authenticated; reads are public.

## Quick start

### 1. Configure your Nix client

Add the deployed Worker URL to your `nix.conf`:

```ini
substituters = https://<your-worker>.workers.dev https://cache.nixos.org
trusted-public-keys = <your-key-name>:<base64-public-key> cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
```

### 2. Push a store path

```bash
export NIX_TOKEN=<token>
nix copy \
  --to "https://x-auth-token:${NIX_TOKEN}@<your-worker>.workers.dev" \
  /nix/store/<hash>-<name>
```

### 3. Pull on another machine

```bash
nix build nixpkgs#hello  # served from the Worker if cached
```

## HTTP API

| Method | Path              | Auth   | Description                      |
| ------ | ----------------- | ------ | -------------------------------- |
| `GET`  | `/nix-cache-info` | public | Cache metadata (priority, etc.). |
| `GET`  | `/<hash>.narinfo` | public | Narinfo for a store path.        |
| `GET`  | `/nar/<hash>.nar` | public | NAR archive bytes.               |
| `PUT`  | `/<hash>.narinfo` | basic  | Upload a narinfo.                |
| `PUT`  | `/nar/<hash>.nar` | basic  | Upload a NAR archive.            |

**Auth:** HTTP Basic with username `x-auth-token` and password `${NIX_TOKEN}`.

**Signing:** if `NIX_SECRET` is set and the uploaded `.narinfo` has no `Sig:` field, the Worker signs it before storing.

## Configuration

### Environment variables

| Variable             | Required    | Description                                                                                                      |
| -------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------- |
| `NIX_TOKEN`          | for uploads | Password for HTTP Basic auth on `PUT` requests (username is `x-auth-token`).                                     |
| `NIX_SECRET` | no          | `<key-name>:<base64>` — base64 decodes to 64 Ed25519 secret-key bytes (as emitted by `nix key generate-secret`). |

### Bindings

| Binding      | Type      | Description                           |
| ------------ | --------- | ------------------------------------- |
| `NIX_BUCKET` | R2 bucket | Stores `.narinfo` and `.nar` objects. |

## Deployment

Production deployment is via **Terraform** (Cloudflare provider). A Terraform example will be added in a follow-up.

CI publishes the Worker bundle to GitHub Releases:

- `build/index.js`
- `build/index_bg.wasm`

These two files are the minimum required artifacts — `index.js` imports `./index_bg.wasm` at runtime.

> `wrangler.toml` is provided for **local testing only**, not production deploys.

## Development

The Nix flake provides a reproducible dev shell with all required tooling:

```bash
nix develop -c cargo test           # run the test suite
nix develop -c worker-build --dev   # build the Worker bundle into ./build
nix develop -c wrangler dev         # serve locally via wrangler
```

## Dependencies

Runtime crates:

- [`worker`](https://crates.io/crates/worker) / [`worker-macros`](https://crates.io/crates/worker-macros) — Cloudflare Workers Rust SDK
- [`narinfo`](https://crates.io/crates/narinfo) — parse/serialize `.narinfo`
- [`http-auth-basic`](https://crates.io/crates/http-auth-basic) — Basic Auth parsing
- [`ed25519-dalek`](https://crates.io/crates/ed25519-dalek), [`sha2`](https://crates.io/crates/sha2), [`base64`](https://crates.io/crates/base64) — `.narinfo` signing + validation

Tooling:

- **Nix** — reproducible dev environment
- **worker-build** — produces the JS + WASM bundle (from Cloudflare's `workers-rs`)
- **wrangler** — local testing
- **Terraform** — production deployment

## License

[MIT](LICENSE)
