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

resource "cloudflare_zero_trust_access_policy" "allowed_user" {
  account_id       = var.cloudflare_account_id
  name             = "${var.worker_name}-${var.environment}-allow-user"
  decision         = "allow"
  session_duration = var.access_session_duration

  include = [{
    email = {
      email = var.access_allowed_email
    }
  }]
}

resource "cloudflare_zero_trust_access_application" "worker" {
  account_id = var.cloudflare_account_id
  name       = "${var.worker_name}-${var.environment}"
  type       = "self_hosted"
  domain     = local.worker_domain

  policies = [{
    id         = cloudflare_zero_trust_access_policy.allowed_user.id
    precedence = 1
  }]

  depends_on = [cloudflare_worker.graphql]
}
