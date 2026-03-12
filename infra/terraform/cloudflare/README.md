# Cloudflare Worker Terraform

This stack manages the Cloudflare-side infrastructure for the `gql-async-graphql` Worker:

- the Worker shell and workers.dev subdomain settings
- Worker observability settings
- a Cloudflare Access application and reusable policy protecting the `workers.dev` hostname

Wrangler still deploys the Worker code. Terraform does not upload the Worker bundle in this setup.

## Prerequisites

- Terraform `>= 1.8`
- A Cloudflare API token with Worker and Access write access for the target account
- An existing R2 bucket for Terraform state
- R2 API credentials for the state bucket

## Files

- `versions.tf`: Terraform / provider constraints and the empty `s3` backend block
- `backend.hcl.example`: Example backend config for an R2-backed `terraform init`
- `terraform.tfvars.example`: Example non-secret input values

## Required Environment Variables

Provider authentication:

```sh
export CLOUDFLARE_API_TOKEN=...
```

Terraform inputs:

```sh
export TF_VAR_cloudflare_account_id=...
export TF_VAR_workers_dev_account_subdomain=...
export TF_VAR_access_allowed_email=...
```

R2 backend credentials:

```sh
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
```

## Bootstrap the R2 Backend

The state bucket and its credentials need to exist before Terraform can use the remote backend.
For this hobby project, creating the bucket and API token outside Terraform is acceptable.

Copy `backend.hcl.example` to `backend.hcl` and fill in the real values.

If the Worker or Access resources already exist in Cloudflare, import them before the first apply
instead of trying to recreate them blindly.

## Local Workflow

Initialize against the remote R2 backend:

```sh
terraform init -backend-config=backend.hcl
```

Validate and preview:

```sh
terraform fmt -check
terraform validate
terraform plan
```

## What Gets Protected

The Access application targets:

```text
https://<worker_name>.<workers_dev_account_subdomain>.workers.dev
```

The first-pass policy allows a single email address. If you later need automation clients,
add Access service tokens rather than opening the endpoint publicly.
