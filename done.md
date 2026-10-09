# infra — completed tasks

### T5. Add the required project files

The project was missing `AGENTS.md`, `done.md`, `roadmap.md`, `ideas.md`, and `toolchain.md`. Done means: all files exist, with the real toolchain (actionlint via mise, git, GitHub Actions) in `toolchain.md`.

Those files are in the repo root. `toolchain.md` lists actionlint, git, and the Rust pin used by `ci-rust.yml`. `tests/fixtures/rust-crate/src/lib.rs` already has a unit test, so no new test was added.
