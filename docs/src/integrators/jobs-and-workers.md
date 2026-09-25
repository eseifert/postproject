# Jobs and workers

PostProject persists the production meaning of requested work. Your host,
service, or script remains responsible for choosing jobs, starting processes,
managing resources, and applying retry policy.

## Request work

Create the job and any parameter metadata in one transaction. The request names
its input representations and the desired output asset and representation kind.
Parameters use the normal typed metadata model with the job as their target.

Listing jobs is a snapshot read in stable identity order. Use the semantic
revision feed to discover that a job changed, then reload it for complete state.

## Implement a worker

A worker follows the same protocol on every public surface:

1. Claim a requested job with tool attribution, optional agent attribution,
   caller-supplied current time, and a lease expiry.
2. Retain the returned claim token privately.
3. Renew the claim before expiry when work takes longer than one lease.
4. On success, stage the output representation and activity, then complete the
   job in that same transaction.
5. On a tool error, fail the job with a bounded diagnostic. To abandon work
   without recording failure, release the claim.

Two processes cannot hold a valid claim on the same job. An expired lease is
claimable again when a later claimant supplies a current time at or beyond the
expiry. Every token-checked transition rejects a stale or unrelated token.

Completion validates that the activity consumes the job inputs and produces
the staged output requested by the job. Do not commit the representation or
activity in an earlier transaction: only `complete` gives the all-or-nothing
guarantee.

## Public operations

| Operation | Rust | C | C++ | Python | CLI |
|---|---|---|---|---|---|
| request | `request_job` | `pp_transaction_request_job` | `requestJob` | `request_job` | `job request` |
| list | `jobs` | `pp_production_jobs` | `jobs` | `jobs` | `job list` |
| claim | `claim_job` | `pp_transaction_claim_job` | `claimJob` | `claim_job` | `job claim` |
| renew | `renew_job_claim` | `pp_transaction_renew_job_claim` | `renewJobClaim` | `renew_job_claim` | `job renew` |
| release | `release_job_claim` | `pp_transaction_release_job_claim` | `releaseJobClaim` | `release_job_claim` | `job release` |
| complete | `complete_job` | `pp_transaction_complete_job` | `completeJob` | `complete_job` | `job complete` |
| fail | `fail_job` | `pp_transaction_fail_job` | `failJob` | `fail_job` | `job fail` |
| cancel | `cancel_job` | `pp_transaction_cancel_job` | `cancelJob` | `cancel_job` | `job cancel` |
| plan regeneration | `plan_regeneration` | `pp_production_plan_regeneration` | `planRegeneration` | `plan_regeneration` | `job plan` |

The C ABI returns owned job and regeneration-plan sets. Release every returned
set with its matching release function. The C++ and Python bindings copy result
values into their native immutable types.

The CLI completion adapter creates a single-file output. Library callers can
stage any supported representation structure—single resource, image sequence,
ordered parts, or package—before calling the same completion operation.

## Plan and explicitly enqueue regeneration

`plan_regeneration` accepts artifact representation IDs and returns proposals
derived from recorded producers. Each proposal includes a fresh requested job
and parameter metadata already retargeted to that job ID. Planning requires one
unambiguous producer for each artifact.

The proposal is not durable. To enqueue it, open a transaction, request the
proposed job, copy its parameter assertions to the job target, and commit. This
separation lets a host review, prioritize, modify, or discard proposed work and
prevents a read from creating background work.

The command-line equivalent is read-only:

```console
postproject --json job plan production.pproj \
  --artifact 0195f1d8-6aa5-7f00-8000-000000000001
```

See [jobs and production work](../concepts/jobs.md) for the domain model and
[revision feed](revision-feed.md) for cross-process change discovery.
