# 2026-10-17 — Technical writing

## Changes

Added this direction to AGENTS.md, the conventions, and a new writing guide:
"Write this 80% of the way to ASD-STE100."
The target means practical clarity. It is not a measured score or a claim of formal compliance.
The guide applies to specifications, runbooks, READMEs, release notes, verification records, OpenAPI descriptions, and code comments.

Revised existing technical prose into shorter statements with explicit actors, conditions, units, and results.
Kept commands, identifiers, limits, measurements, and historical versions exact.
Kept dated verification records tied to their original milestone.
Changed generated OpenAPI descriptions through src/contracts.rs.

## Decisions and constraints

The writing direction applies to Korean text too. It does not require a language change.
Keep technical terms when a simpler word would change the meaning.
Do not add an arbitrary word-count check or claim formal ASD-STE100 compliance.

Astra identified two descriptions that needed correction.
The supervisor must retain the admission permit for submitted file operations, including operations that have not started.
Only image-data chunks have a fixed 64 KiB maximum. The manifest is sent as one chunk and grows with the output count.
Corrected both descriptions to preserve these conditions.

## Validation

- Windows `just check`: passed formatting, warnings-denied clippy, 32 tests, and generated-contract drift checks.
- Generated OpenAPI: matched the previous document after removing description fields. Paths, schemas, versions, and numeric constraints stayed unchanged.
- Source comparison: Python syntax trees and Rust/JavaScript code stayed unchanged outside OpenAPI description strings and formatting.
- Documentation comparison: existing fenced command blocks and historical measurement tables stayed unchanged. Raw benchmark reports and image fixture data stayed unchanged.
- Fixture integrity: all 38 hashes passed. Updated only the hash of the revised fixture README.
- Python script compilation, Node syntax checks, relative Markdown links, and Git whitespace checks: passed.
- Astra reviewed the behavior, operations, architecture, benchmark, deployment, agent instructions, and relevant native/cancellation comments. Corrected both reported wording issues.

Remote CI results belong to the run for the integrated commit.
This change did not repeat container measurements or publish a release.
