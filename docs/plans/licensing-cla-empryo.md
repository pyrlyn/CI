# Plan: dual licensing, CLA and Empryo ideas for rtok/cox

Plan for rtok and cox: licensing, CLA and ideas from Empryo. Drafted 2026-10-02; statuses are updated as work progresses. PR links will be added once the PRs are opened.

## 1. Dual license and license file sync — 🟡 in progress

- [ ] Reference files `LICENSE-GPL` (GPL-3.0) and `LICENSE-COMMERCIAL` live in `pyrlyn/infra`. This is the single source of truth.
- [ ] A workflow in infra (`license-sync.yml`). When the reference changes, the workflow opens or updates an automated PR in every GPL repository of the org, the way Dependabot does. If branch protection allows it, the PR merges itself.
- [ ] Reusable workflow `license-check.yml`. CI in a GPL repository fails if either of the two files differs from the reference.
- [ ] The README of every GPL repository gets, in the same PR, the line "Commercial use requires a separate paid license — see LICENSE-COMMERCIAL". The line is held by `license-sync` markers. If the README has translations, the line is added to them too.
- [ ] Non-GPL repositories (MIT and others, judged by LICENSE and the `license` field in Cargo.toml or package.json) are skipped. They will be listed in the report; Ivan decides on them.
- Where: `pyrlyn/infra` plus a PR in every GPL repository. Nothing may be merged without Ivan's decision.

## 2. License headers in source files — 🟡 in progress (draft PR)

- [ ] Each GPL repository gets a separate draft PR. The file header contains the year, `Ivan Tugay`, `SPDX-License-Identifier: GPL-3.0-or-later`, the phrase "Licensed under GPL-3.0 or later" and a link to the full text.
- [ ] Generated, vendored and fixture files, lock files, JSON and files that already have a header are skipped.
- [ ] ⚠️ If Cargo.toml says `GPL-3.0-only` rather than `-or-later`, the PR flags it. Ivan decides; nothing is changed silently.
- [ ] `OR LicenseRef-Commercial` in the SPDX expression can be proposed. For now this is only a proposal and is not applied.
- [ ] Optional: a header presence check in `license-check.yml`, off by default.

## 3. CLA — 🟡 in progress (draft PR)

- [ ] Text in `CLA.md` (English, primary) and `CLA.ru.md` (Russian; in case of discrepancy the English text prevails). The contributor grants Ivan Tugay a perpetual, irrevocable license to use, relicense and commercially use the contribution, including a patent license. The contribution itself stays under the GPL for everyone. A lawyer must review the text before it is applied.
- [ ] Signing via the CLA Assistant Lite GitHub Action (`contributor-assistant/github-action`): a reusable workflow in infra and a caller workflow in GPL repositories. The GitHub account serves as the signature; an unsigned PR is flagged. Bots are allowlisted.
- [ ] Manual, Ivan: create a secret with a token, choose where signatures are stored (a branch in infra or a separate repository) and make the CLA check required via rulesets. Instructions will be in `docs/cla.md` in infra.

## 4. Ideas from Empryo for rtok and cox — 🟢 research done, 🟡 detailed design in progress, ⚪ implementation not started

Empryo (proxysoul/Empryo) is distributed under **BSL 1.1**, so its code must not be copied. We take only the ideas and rewrite them in Rust from scratch.

14 ideas that carry over to Rust:
1. Personalized PageRank over files: weights by the focus of the turn, warm start, type-only edges weigh 0.3×.
2. IDF- and confidence-aware edge weights: import vs. name match, common names like `new` are cut off.
3. Pairs of files that change together, from `git log` (300 commits, cached by HEAD).
4. Coverage label `(→N)` and a map budget: blocks are chosen by binary search, cached.
5. Trigram prefilter for grep.
6. Duplicate search via MinHash. In Empryo, PR #220 sped up the cold scan about 12× (33.2 → 2.7 s).
7. Answering grep from the symbol index.
8. Diagnostics delta after an edit.
9. Fallback chain LSP → AST → tree-sitter → regex, and cleanup of hung LSP servers.
10. Composite tools `rename_symbol` and `project`.
11. Context compaction without a model call (cumulative session state).
12. RRF-based memory over four signals: words, trigrams, file links, co-changes.
13. Robust edits: fuzzy whitespace, hash guard, undo stack.
14. Command output compression that preserves stack traces.

### Tasks for rtok (⚪ to do)
1. Repository map by PageRank between files and the `(→N)` label. Do not persist personalized scores (see Empryo #228). Size M.
2. IDF and edge confidence in `callers`, `impact` and `explore`. Size S–M.
3. Co-change table for the map, `impact`, `affected` and memory. Size M.
4. Answer grep from the symbol index instead of a full block in the guard. Size S–M.
5. Linking memory notes to files (`note_files`) and two new RRF signals. Size S–M.
6. Trigram prefilter for `read search`, with a check that it actually wins. Size M.
7. `rtok graph dupes` on MinHash and linking tests to code by file names. Size M.

### Tasks for cox (⚪ to do)
1. Repository map by graph and PageRank instead of sorting by recency, personalized by git only. Possibly a crate shared with rtok. Size L/M.
2. Compaction without a model call for the "files" and "errors" sections. Size M.
3. Diagnostics delta in the output of `edit` and `write`. Size M.
4. Navigation via LSP and `rename_symbol`. Size L.
5. A `check` tool: tests, build, linter. Size M.
6. `read` and `edit` by symbol. Size M.
7. Collapsing repeats in output via rtok's `cmd` rules. Size S.

Detailed design is under way now: where each task fits in the code, how to do it in Rust, acceptance criteria and what to take first. The result will be added here.

## 5. Dependency license scanning — ⏸ paused, [license-scanning.md](license-scanning.md)

- Original request: actively check only GPL, skip MIT and Apache.
- Open question: fail CI on a GPL dependency or only show it in the report?
- ⚠️ A GPL dependency breaks the commercial license of rtok and cox.
- Proposal: `cargo-deny` in infra. Leave MIT and Apache alone, show GPL in the report and fail CI on it.
