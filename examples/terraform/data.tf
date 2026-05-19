data "github_release" "worker" {
  repository  = var.github_repo
  owner       = var.github_owner
  retrieve_by = "latest"
}

locals {
  github_release_asset_urls = {
    for item in data.github_release.worker.assets : item.name => item.browser_download_url
  }
}

data "http" "index_js" {
  url = local.github_release_asset_urls["index.js"]
}

resource "local_file" "index_js" {
  filename = "${path.module}/.terraform/index.js"
  content  = data.http.index_js.response_body
}

data "http" "index_wasm" {
  url = local.github_release_asset_urls["index_bg.wasm"]
}

resource "local_file" "index_wasm" {
  filename       = "${path.module}/.terraform/index_bg.wasm"
  content_base64 = data.http.index_wasm.response_body_base64
}
