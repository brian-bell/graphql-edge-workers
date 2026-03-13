# Cloudflare Worker Terraform

This stack manages the Cloudflare-side infrastructure for the `gql-async-graphql` Worker:

- the Worker shell and workers.dev subdomain settings
- Worker observability settings

Wrangler still deploys the Worker code. Terraform does not upload the Worker bundle in this setup.

## Prerequisites

- Terraform `>= 1.8`
- A Cloudflare API token with Worker write access for the target account
- An existing R2 bucket for Terraform state
- R2 API credentials for the state bucket
- The account-level `workers.dev` subdomain value for the Cloudflare account

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
export TF_VAR_worker_name=gql-async-graphql
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

If the Worker resource already exists in Cloudflare, import it before the first apply instead of
trying to recreate it blindly.

## GitHub Actions Secrets

The `gql-async-graphql` workflow expects these secrets in the `cloudflare` GitHub environment:

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`
- `CLOUDFLARE_WORKERS_DEV_SUBDOMAIN`
- `R2_STATE_BUCKET`
- `R2_STATE_ACCESS_KEY_ID`
- `R2_STATE_SECRET_ACCESS_KEY`

The workflow uses those values to:

- initialize the remote R2 backend
- run Terraform plan on pull requests and post a sticky PR comment
- run Terraform apply on non-PR executions before the deploy step

The Terraform job is attached to the `cloudflare` GitHub environment and only runs when files under
`infra/terraform/cloudflare/**` change, unless the workflow is started manually with
`workflow_dispatch`.

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

If you are using exported `TF_VAR_*` environment variables, `terraform plan` will pick them up
automatically. If you prefer a local variables file, copy `terraform.tfvars.example` to your own
untracked file and pass it with `-var-file`.

## workers.dev Access

Cloudflare Access for `workers.dev` is not managed by this Terraform stack.

Use the Cloudflare dashboard after the Worker exists:

1. Open `Workers & Pages`
2. Select the Worker
3. Open `Settings > Domains & Routes`
4. Enable Cloudflare Access for the `workers.dev` hostname

This is separate from the generic Zero Trust Access application API used for custom domains.

## CI Container

GitHub Actions runs the worker workflow inside a prebuilt container image so Rust, the WASM target,
Node/npm, and Terraform do not need to be installed during every job.

- Dockerfile: `.github/docker/gql-async-graphql-ci/Dockerfile`
- Image workflow: `.github/workflows/ci-image-gql-async-graphql.yml`

The worker workflow expects the `latest` tag for that image to exist in GHCR. Publish the image
workflow once before relying on the containerized worker workflow.
