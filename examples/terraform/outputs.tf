output "worker_name" {
  value       = cloudflare_worker.nix_cache.name
  description = "Deployed Worker script name. Pair with your account's *.workers.dev subdomain to construct the cache URL."
}

output "r2_bucket" {
  value       = cloudflare_r2_bucket.nix.name
  description = "R2 bucket backing the cache."
}

output "release_tag" {
  value       = data.github_release.worker.release_tag
  description = "GitHub release tag actually resolved and deployed."
}
