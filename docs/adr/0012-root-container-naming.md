# ADR 0012: Root container naming

- Status: Accepted
- Date: 2026-09-20

## Context

The durable container that holds media identity, representations, metadata, provenance, and
revisions is named `Project`: `pp_project_*` in the C ABI, a `projects` table, `ProjectId`
in the domain model, and the `.pproj` extension.

Every candidate host already owns that word. An editor, a compositor, and a digital audio
workstation each call their own file a project, session, or scene, and an integration would
place a PostProject `Project` inside a host `Project`. The resulting sentence — "which
project does this project belong to" — is one an integrator has to decode every time.

The product's own prose does not use the word this way. The roadmap and documentation
describe what PostProject holds as a *production*: production knowledge, shared production
state, two consumers using the same production. The documented example file is
`production.pproj`. The domain type is the one place the word "project" is used for this
concept.

ADR 0001 settles the product, repository, crate, and symbol naming. It does not address the
domain object, and the terminology reference has no entry distinguishing the two senses.

## Decision

The durable root container is `Production`. One production may serve several host projects,
sessions, or scenes that participate in the same body of work.

The rename applies throughout the current model: `ProductionId`, `Production`,
`pp_production_t`, `pp_production_*`, the production storage table, object-reference kinds,
the C++ and Python APIs, CLI terminology, and documentation. Pre-1.0 interfaces and schemas
receive no `Project` compatibility aliases.

The `.pproj` extension remains. It identifies the PostProject storage format and product,
not the root domain type, and is already clear in names such as `production.pproj`.

## Consequences

Host integrations can now distinguish their own project/session/scene from the shared
production without a permanent translation step. The immediate cost is a breaking rename
across the domain, persistence, C ABI, bindings, fixtures, examples, and documentation.

The PostProject product and repository retain their names, and native library artifacts
remain `libpostproject` as decided by ADR 0001.
