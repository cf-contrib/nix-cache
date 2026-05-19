variable "account_id" {
  type        = string
  description = "Cloudflare account ID that owns the Worker and R2 bucket."
}

variable "worker_name" {
  type        = string
  description = "Cloudflare Worker script name."
  default     = "cf-nix-cache"
}

variable "worker_compatibility_date" {
  type        = string
  description = "Workers runtime compatibility date."
  default     = "2026-05-14"
}

variable "r2_bucket_name" {
  type        = string
  description = "R2 bucket name used to store .narinfo and .nar objects."
}

variable "nix_token" {
  type        = string
  description = "Password for HTTP Basic auth on Worker upload endpoints (username is x-auth-token)."
  sensitive   = true
}

variable "nix_secret" {
  type        = string
  description = "Ed25519 signing secret as <key-name>:<base64> (as emitted by `nix key generate-secret`). The Worker rejects unsigned narinfo, so this is required unless every uploader pre-signs."
  sensitive   = true
}

