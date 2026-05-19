# Deploying nix-cache with Terraform

Deploys the Worker bundle published to this repo's GitHub Releases to Cloudflare
Workers + R2.

The bundle (`index.js` + `index_bg.wasm`) is downloaded with the **GitHub**
provider and uploaded as a multi-module ESM Worker with the **Cloudflare**
provider — no `wrangler` or local build step required.

## Prerequisites

- Terraform >= 1.9.
- A Cloudflare account with Workers and R2 enabled.
- A Cloudflare API token with **Workers Scripts: Edit** and **Workers R2 Storage: Edit** scopes,
  exported as `CLOUDFLARE_API_TOKEN`.
- (Optional) A GitHub token exported as `GITHUB_TOKEN` if you hit anonymous API rate limits.

## Usage

```bash
cp terraform.tfvars.example terraform.tfvars
# edit terraform.tfvars: set cloudflare_account_id, r2_bucket_name, nix_token

export CLOUDFLARE_API_TOKEN=...

terraform init
terraform plan
terraform apply
```

## Upgrading to a new release

The example always tracks the **latest** GitHub release. After a new release is
published, re-run `terraform apply` — Terraform diffs the downloaded asset
content, uploads a new Worker version, and shifts 100% of traffic to it.

## Pointing Nix at the deployed cache

After `terraform apply` succeeds, the Worker is reachable at
`https://<worker_name>.<your-account-subdomain>.workers.dev`. Add it to your
`nix.conf`:

```ini
substituters = https://cf-nix-cache.<your-subdomain>.workers.dev https://cache.nixos.org
trusted-public-keys = <key-name>:<base64-public-key> cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
```

Push a store path:

```bash
nix copy \
  --to "https://x-auth-token:${NIX_TOKEN}@cf-nix-cache.<your-subdomain>.workers.dev" \
  /nix/store/<hash>-<name>
```

## Inputs

| Variable                | Required | Default        | Description                                                                |
| ----------------------- | -------- | -------------- | -------------------------------------------------------------------------- |
| `cloudflare_account_id` | yes      | —              | Cloudflare account ID.                                                     |
| `r2_bucket_name`        | yes      | —              | R2 bucket name to create (or import) for cache storage.                    |
| `nix_token`             | yes      | —              | Password for HTTP Basic auth on PUT uploads.                               |
| `nix_secret`            | yes      | —              | `<key-name>:<base64>` Ed25519 signing secret. Required because the Worker rejects unsigned narinfo. |
| `worker_name`           | no       | `cf-nix-cache` | Cloudflare Worker script name.                                             |
| `compatibility_date`    | no       | `2026-05-14`   | Workers runtime compatibility date.                                        |
| `github_owner`          | no       | `cf-contrib`   | GitHub owner hosting the release.                                          |
| `github_repo`           | no       | `nix-cache`    | GitHub repository hosting the release.                                     |

## Notes

- State is local by default — add a backend block to `versions.tf` for shared/CI use.
- `terraform.tfvars` and `.build/` are gitignored; only `terraform.tfvars.example` is committed.
- Worker bindings are reset on every version upload. Any secret you want to keep
  across applies must be declared as a binding in this module.
