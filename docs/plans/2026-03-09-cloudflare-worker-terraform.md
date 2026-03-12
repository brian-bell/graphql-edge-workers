# Cloudflare Worker Terraform Plan

**Date:** 2026-03-09
**Status:** Proposed

## Goal

Add a small Terraform stack for the Cloudflare Worker infrastructure used by `gql-async-graphql`, and extend the existing GitHub Actions workflow so pull requests show a Terraform plan comment. A future Terraform apply should run before the existing deploy step, but both apply and deploy remain disabled for now.

## Scope

This plan is intentionally narrow:

- Manage long-lived Cloudflare Worker infrastructure with Terraform
- Protect the `workers.dev` endpoint with Cloudflare Access
- Keep `wrangler deploy` as the code deployment mechanism
- Add PR-time Terraform validation and plan visibility in GitHub Actions
- Add a disabled Terraform apply step before the existing disabled deploy step

Out of scope for the first pass:

- Custom domains or DNS records
- IP allowlist firewall rules
- Terraform-managed Worker secrets
- Replacing `wrangler deploy` with Terraform-based code deployment
- Paid observability sinks or complex alerting

## Current State

- The active worker is `workers/gql-async-graphql`
- `workers/gql-async-graphql/wrangler.toml` defines the Worker name and a non-secret `ORIGIN_BASE_URL` var
- `.github/workflows/gql-async-graphql.yml` already runs test and build steps
- The deploy step is present but disabled with `if: false`

## Proposed Ownership Boundary

Keep the ownership split simple:

- Terraform owns the Worker's Cloudflare-side infrastructure shell and baseline settings
- Wrangler continues to upload the built Worker code
- Worker request behavior remains in Rust
- Sensitive runtime configuration stays out of Terraform state

This avoids having two tools both trying to own the same Worker code deployment while still giving the project declarative infrastructure.

## Terraform Design

### Directory Layout

Use a single stack rather than modules:

```text
infra/
  terraform/
    cloudflare/
      versions.tf
      providers.tf
      variables.tf
      main.tf
      outputs.tf
      README.md
      terraform.tfvars.example
```

For a hobby project with one Worker and one environment, a single stack is simpler than a module hierarchy.

### Managed Resources

Recommended initial resources:

- `cloudflare_worker`
  - Worker name
  - `workers.dev` subdomain enablement
  - basic tags
  - basic observability settings
- Cloudflare Access application and policy protecting the `workers.dev` endpoint
  - single-user access is acceptable for the first pass
  - machine-to-machine tokens can wait until needed

Possible later resources, but not for the first pass:

- `cloudflare_workers_custom_domain` once there is a real zone and hostname
- route resources if the project moves off `workers.dev`
- WAF custom rules for IP allowlisting once traffic is served through a Cloudflare zone

### Suggested Variables

- `cloudflare_account_id`
- `worker_name`
- `enable_workers_dev_subdomain`
- `observability_enabled`
- `observability_head_sampling_rate`
- `environment`

### Suggested Outputs

- `worker_name`
- `workers_dev_url` when subdomain is enabled

## Security Baseline

Keep the first version conservative:

- Use a dedicated Cloudflare API token limited to the target account's Worker management scope
- Require Cloudflare Access in front of the `workers.dev` endpoint
- Store the Cloudflare token in GitHub Actions secrets, not in Terraform files
- Do not put sensitive values in `wrangler.toml` `vars`; Cloudflare recommends secrets for sensitive data
- Do not manage Worker secrets in Terraform for now
- Scope `GITHUB_TOKEN` permissions to the minimum required values in the workflow
- Avoid `pull_request_target` for Terraform plan execution against PR code
- Defer IP allowlist rules until the Worker is moved behind a custom domain in a Cloudflare zone

Two practical implications follow from that:

1. Terraform plan comments should only run for same-repository pull requests, where GitHub secrets are available without using `pull_request_target`.
2. Forked pull requests should still run Terraform formatting and validation, but should skip cloud-authenticated plan generation and PR commenting.

## Monitoring Baseline

Keep monitoring basic and free-tier friendly:

- Enable Worker observability in Terraform if supported by the provider configuration used during implementation
- Enable invocation logs
- Use full sampling at first because traffic should be low
- Keep `/health` as the simple liveness check
- Add a post-deploy smoke test later when deploy is enabled

Do not add paid sinks, third-party APM, or log shipping in the first pass.

## Secrets and Configuration Strategy

`ORIGIN_BASE_URL` is currently a plain variable in `wrangler.toml`, which is acceptable only because it is not sensitive.

For the first Terraform pass:

- Leave `ORIGIN_BASE_URL` where it is
- Keep user authentication at the Cloudflare Access layer rather than adding Worker-level auth logic
- Reserve Terraform for infrastructure settings, not runtime secret management

If sensitive bindings are added later, they should move to Cloudflare Worker secrets rather than plain Terraform variables.

## State Strategy

This is the main design decision that should stay explicit.

### Recommendation

Use Cloudflare R2 as the shared Terraform backend, and do not enable CI apply until that backend is configured and tested.

### Reasoning

- A disabled apply step is fine with local state
- An enabled apply step in GitHub Actions without shared state is unsafe and will drift quickly
- R2 keeps state storage inside Cloudflare instead of adding another provider
- Terraform can use R2 through the standard `s3` backend configuration

### Backend Shape

Recommended backend approach:

- One dedicated R2 bucket for Terraform state
- One state key for this stack, such as `prod/cloudflare-worker.tfstate`
- Backend auth via R2 access key ID and secret access key
- Enable backend locking with the Terraform S3 backend lockfile support

Implementation guidance:

- Keep backend credentials out of committed Terraform files
- Prefer partial backend configuration in code plus `-backend-config` values from local env or CI secrets
- Document the exact `terraform init` contract in the Terraform README

### First-Pass Behavior

- Commit Terraform configuration
- Ignore `.terraform/`, `*.tfstate`, and crash logs
- Let CI run formatting, validation, and advisory planning
- Keep apply disabled until state handling is finalized

## GitHub Actions Plan

Update `.github/workflows/gql-async-graphql.yml` so it also triggers on Terraform changes:

- `infra/terraform/cloudflare/**`

Recommended job flow:

1. Checkout
2. Install Rust toolchain
3. Run existing Rust test/build steps
4. Install Terraform
5. `terraform fmt -check`
6. `terraform init`
7. `terraform validate`
8. `terraform plan` on pull requests from the same repository
9. Post or update a PR comment with the plan summary
10. Keep `terraform apply` disabled
11. Keep `wrangler deploy` disabled

### PR Comment Strategy

Use the normal `pull_request` event, not `pull_request_target`.

Recommended behavior:

- Same-repo PR: run `terraform plan`, write the plan to a file, and post or update a sticky PR comment
- Fork PR: skip authenticated plan generation and comment posting, but still run `fmt` and `validate`
- All runs: upload the plan text as an artifact or write it to the job summary for debugging

### Workflow Permissions

At minimum:

- `contents: read`
- `pull-requests: write`

No broader repository permissions should be necessary for the plan comment path.

### Apply / Deploy Ordering

When the workflow is eventually enabled, the intended sequence should be:

1. Terraform apply
2. Wrangler deploy
3. Optional smoke test

For now, mirror the existing deploy posture:

- `Terraform Apply` step present but disabled with `if: false`
- `Deploy to Cloudflare` remains disabled with `if: false`

That sequencing should be visible in the workflow even before enablement, so there is no ambiguity later.

## Files Expected in Implementation

- Create: `infra/terraform/cloudflare/versions.tf`
- Create: `infra/terraform/cloudflare/providers.tf`
- Create: `infra/terraform/cloudflare/variables.tf`
- Create: `infra/terraform/cloudflare/main.tf`
- Create: `infra/terraform/cloudflare/outputs.tf`
- Create: `infra/terraform/cloudflare/README.md`
- Create: `infra/terraform/cloudflare/terraform.tfvars.example`
- Modify: `.github/workflows/gql-async-graphql.yml`
- Modify: `.gitignore`

## Phased Task List

### Task 0: Confirm Terraform ownership and state approach

- Confirm the R2 bucket name, state key, and credential naming
- Document that apply stays disabled until the R2 backend is configured and tested
- Keep Wrangler as the only code deployment tool

### Task 1: Scaffold the Terraform stack

- Add the `infra/terraform/cloudflare` directory
- Pin Terraform and Cloudflare provider versions
- Add provider configuration and input variables

### Task 2: Model the Worker infrastructure

- Create the Worker resource with name, tags, subdomain settings, and observability
- Add Cloudflare Access resources to protect the `workers.dev` endpoint
- Add outputs for operator visibility

### Task 3: Add local/operator documentation

- Document required environment variables and secrets
- Add a simple `terraform.tfvars.example`
- Document the R2 backend bootstrap and `terraform init` flow
- Update `.gitignore` for Terraform local artifacts

### Task 4: Add CI validation and PR plan comments

- Extend workflow triggers to include Terraform paths
- Add Terraform setup, fmt, init, validate, and plan steps
- Post or update a sticky PR comment on same-repo pull requests

### Task 5: Add disabled apply before disabled deploy

- Insert a `Terraform Apply` step before `Deploy to Cloudflare`
- Disable it with the same explicit `if: false` pattern used today
- Leave a short comment explaining that apply is blocked on backend/secrets readiness

### Task 6: Verify and document the rollout path

- Validate Terraform locally
- Confirm the workflow still builds the Rust Worker
- Record the exact conditions required before enabling apply and deploy

## Risks and Open Questions

- Shared Terraform state is the real blocker for enabling apply
- R2 backend bootstrap is slightly circular because the backend bucket should exist before Terraform starts using it; for a hobby project, creating the bucket and API token out-of-band is acceptable
- Cloudflare provider support for some Worker observability fields may require trimming the first version to the simplest supported subset
- Cloudflare Access resources may require a small amount of account-specific setup outside this repo depending on the chosen identity method
- If the project later wants Terraform to manage bindings or script versions, it will overlap with Wrangler and should be treated as a separate migration
- If a custom domain is added later, zone resources and certificate behavior should be planned separately
- IP allowlist firewall rules are intentionally deferred because they fit better once traffic is on a custom domain in a managed zone

## Recommended Implementation Order

1. Land Terraform files and local docs
2. Land CI formatting, validation, and PR plan comments
3. Add the disabled apply step before the disabled deploy step
4. Revisit backend/state before enabling apply
5. Revisit deployment ownership only if Wrangler becomes limiting
