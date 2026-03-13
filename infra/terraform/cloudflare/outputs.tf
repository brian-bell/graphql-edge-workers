output "worker_name" {
  description = "Worker name managed by Terraform."
  value       = cloudflare_worker.graphql.name
}

output "workers_dev_url" {
  description = "workers.dev URL for the GraphQL worker."
  value       = var.enable_workers_dev_subdomain ? "https://${local.worker_domain}" : null
}
