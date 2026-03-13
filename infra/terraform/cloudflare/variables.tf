variable "cloudflare_account_id" {
  description = "Cloudflare account ID that owns the Worker resources."
  type        = string
}

variable "worker_name" {
  description = "Worker name. This should stay aligned with workers/gql-async-graphql/wrangler.toml."
  type        = string
  default     = "gql-async-graphql"
}

variable "workers_dev_account_subdomain" {
  description = "Account-level workers.dev subdomain prefix, without the trailing .workers.dev."
  type        = string
}

variable "environment" {
  description = "Environment tag for this stack."
  type        = string
  default     = "prod"
}

variable "enable_workers_dev_subdomain" {
  description = "Whether the Worker should be reachable on workers.dev."
  type        = bool
  default     = true
}

variable "enable_observability" {
  description = "Whether Cloudflare Worker observability should be enabled."
  type        = bool
  default     = true
}

variable "observability_head_sampling_rate" {
  description = "Worker observability head sampling rate between 0 and 1."
  type        = number
  default     = 1

  validation {
    condition     = var.observability_head_sampling_rate >= 0 && var.observability_head_sampling_rate <= 1
    error_message = "observability_head_sampling_rate must be between 0 and 1."
  }
}
