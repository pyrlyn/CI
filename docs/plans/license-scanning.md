# License scanning for org dependencies (paused: GPL rule needs clarification)

## Status

**Paused** (2026-10-02). No code or workflow changes have been made yet; resume from this plan.

## Original request

Add dependency license scanning to the org infrastructure (reusable workflow in `pyrlyn/infra`) and scan the org repositories' dependencies:

- GPL-licensed dependencies: allowed / used.
- MIT- and Apache-licensed dependencies: skipped entirely by the checker, never flagged or blocked.
- Report which dependencies are GPL and which are MIT/Apache; the scanner should only actively enforce the GPL rule.

## Open contradiction

GPL dependencies are described as *allowed*, yet the checker is supposed to *actively monitor/enforce* only GPL. It is unclear what "enforce" means for something that is allowed.

## Open question

Should a GPL dependency:

- [ ] fail CI, or
- [ ] only appear in the report (no failure)?

## Licensing context

`rtok` and `cox` ship under **GPL-3.0 plus a paid commercial license** (see their `PRICING.md`). A GPL dependency would break the commercial license, so the usual policy is the opposite of "allow GPL": block GPL, allow permissive licenses.

## Suggested approach

- `cargo-deny` (`cargo deny check licenses`) as a reusable workflow in `pyrlyn/infra`, called from each Rust repo.
- MIT / Apache-2.0 left untouched: allowed, not flagged.
- GPL-family licenses (GPL / LGPL / AGPL) listed in the job summary report **and** failing CI.
- Final GPL behaviour (fail vs report-only) depends on the answer to the question above.
