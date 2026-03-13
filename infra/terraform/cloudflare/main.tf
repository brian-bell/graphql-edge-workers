locals {
  worker_domain = "${var.worker_name}.${var.workers_dev_account_subdomain}.workers.dev"
}

resource "cloudflare_worker" "graphql" {
  account_id = var.cloudflare_account_id
  name       = var.worker_name

  observability = {
    enabled            = var.enable_observability
    head_sampling_rate = var.observability_head_sampling_rate
  }

  subdomain = {
    enabled = var.enable_workers_dev_subdomain
  }
}
