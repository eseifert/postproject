# ADR 0025: Reference local executor

- Status: Accepted
- Date: 2026-09-25

## Context

ADR 0023 defines durable jobs and a host-neutral worker protocol, but a protocol
alone does not demonstrate that requested work can become an atomic output and
provenance fact. PostProject needs one bounded executor for end-to-end use while
remaining a library rather than a scheduler or media framework.

Linking FFmpeg libraries would add a native ABI, packaging, untrusted-input, and
licensing boundary to every PostProject consumer. ADR 0019 already established
the narrower subprocess boundary for `ffprobe` inspection.

## Decision

`postproject-media` provides an `Executor` adapter boundary and one
`FfmpegExecutor`. It invokes a caller-configurable `ffmpeg` executable with an
explicit argument vector, never a shell, and links no FFmpeg library. It captures
bounded stdout and stderr, enforces a caller-configurable nonzero timeout, and
invokes a caller heartbeat while work is running. A missing executable is an
unavailable capability, not a package startup failure. Nothing is downloaded.

The reference executor accepts exactly one online single-file input and one
machine mapping for the job's logical target root. It supports two open-world job
kinds through a deliberately closed local profile table:

| Job kind | Profiles | Output |
| --- | --- | --- |
| `org.postproject:generate-proxy` | `proxy-720p`, `proxy-1080p` | MPEG-4/AAC MP4 |
| `org.postproject:generate-thumbnail` | `thumbnail-640`, `thumbnail-1280` | JPEG |

The exact profile name is a string value of
`https://postproject.org/ns/executor-parameters/1#profile` on the job. Successful
completion copies all job parameter metadata onto the activity without
normalizing unknown values. The activity identifies `ffmpeg`, its reported
version token, and `https://ffmpeg.org/` as the tool.

Output is written to a claim-scoped temporary filename inside the mapped target
root. Only a successful process producing a non-empty regular file is renamed to
the job-ID-derived final name. Failure and timeout remove the temporary output.
The runner never overwrites an existing final or temporary path.

The CLI `job run` runner owns orchestration. It probes capability before claiming,
claims and renews through `ProductionStoreTransaction`, calls the executor, and
then completes, fails, or releases through the same public protocol available to
other workers. Completion adds the fingerprinted representation, activity,
storage-captured snapshots, copied parameters, and succeeded state in one SQLite
transaction. `job run --once` stops after one eligible job; without `--once`, the
command processes the eligible requested jobs present when the run starts. There
is no background thread or implicit execution.

Rust exposes the subprocess adapter. The CLI exposes the complete local runner.
C, C++, and Python continue to expose the worker protocol but do not wrap this
optional executor.

## Licensing and standards impact

FFmpeg's own [license file](https://ffmpeg.org/doxygen/trunk/md_LICENSE.html)
states that its code is primarily LGPL 2.1-or-later and that optional components
can make a particular build GPL; its [legal page](https://ffmpeg.org/legal.html)
describes obligations for programs linked with its libraries. PostProject does
not link, bundle, modify, or redistribute FFmpeg. It only invokes a separately
installed command selected by the user. Consequently the PostProject artifacts
remain `MIT OR Apache-2.0`; the provider of an FFmpeg binary remains responsible
for that binary's configuration, redistribution terms, codec availability, and
any patent considerations. This records the project boundary and is not legal
advice.

The executor introduces no mapping to a media, metadata, identifier, or
provenance standard. Job kinds and profiles are PostProject-controlled terms,
and unknown external metadata remains open-world and lossless.

## Alternatives considered

- **Link FFmpeg libraries.** Rejected because it makes every native package carry
  a larger ABI and licensing boundary.
- **Ship or download FFmpeg.** Rejected because PostProject should not choose or
  redistribute a platform-specific FFmpeg build.
- **Execute through a shell.** Rejected because arguments and media paths must
  not become shell syntax.
- **Give the executor private SQLite access.** Rejected because a reference
  worker must prove the public claim/renew/complete contract used by hosts.
- **Wrap the executor in the C ABI, C++, and Python.** Rejected because hosts
  are expected to act as workers with their own task systems through the
  public job protocol, which every surface already exposes; the executor
  exists to demonstrate that protocol end to end, and the CLI runner does so
  for installed consumers. Exporting it would also cost more than it returns:
  the executor blocks for the length of an `ffmpeg` run and calls a caller
  heartbeat, which the C ABI could carry only as a function-pointer callback,
  a construct it does not export, or as a new asynchronous handle with polling
  and cancellation. And it would fix the closed profile table, the eligibility
  rules, and the subprocess settings into the native compatibility surface,
  where they would be subject to integration-subset rules (ADR 0020) rather
  than remaining adapter details. Python and other tools can invoke the
  installed `postproject job run` command instead.
- **Store artifact state or executor profiles in the core model.** Rejected
  because artifact state remains derived and profiles are typed parameters of
  one adapter, not universal representation semantics.

## Consequences

An installed CLI can turn a requested proxy or thumbnail job into a durable
artifact without Cargo or an application host. Hosts can reuse the Rust adapter
or ignore it and implement the same public worker protocol. Only jobs with a
known profile, one local single-file input, and an explicitly mapped target root
are eligible. Compound inputs, additional codecs, distributed scheduling,
retries, and background execution remain outside this executor.
