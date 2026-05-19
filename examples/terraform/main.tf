# Worker script (logical resource — no content yet).
resource "cloudflare_worker" "nix_cache" {
  account_id = var.account_id
  name       = var.worker_name

  subdomain = {
    enabled = true
  }
}

# Upload a new version on every asset / binding change. The modules list
# carries both the JS entry and the wasm module, matching what worker-build
# emits and what `index.js` imports via `./index_bg.wasm`.
resource "cloudflare_worker_version" "nix_cache" {
  account_id         = var.account_id
  worker_id          = cloudflare_worker.nix_cache.id
  compatibility_date = var.compatibility_date
  main_module        = "index.js"

  modules = [
    {
      name           = "index.js"
      content_type   = "application/javascript+module"
      content_base64 = base64encode(data.http.index_js.response_body)
    },
    {
      name           = "index_bg.wasm"
      content_type   = "application/wasm"
      content_base64 = data.http.index_wasm.response_body_base64
    },
  ]

  bindings = [
    {
      name        = "NIX_BUCKET"
      type        = "r2_bucket"
      bucket_name = cloudflare_r2_bucket.nix.name
    },
    {
      name = "NIX_TOKEN"
      type = "secret_text"
      text = var.nix_token
    },
    {
      name = "NIX_SECRET"
      type = "secret_text"
      text = var.nix_secret
    },
  ]
}

# Promote the new version to 100% of traffic.
resource "cloudflare_workers_deployment" "nix_cache" {
  account_id  = var.account_id
  script_name = cloudflare_worker.nix_cache.name
  strategy    = "percentage"

  versions = [
    {
      percentage = 100
      version_id = cloudflare_worker_version.nix_cache.id
    },
  ]
}

# R2 bucket for cached .narinfo / .nar objects.
resource "cloudflare_r2_bucket" "nix" {
  account_id = var.account_id
  name       = var.r2_bucket_name
}
