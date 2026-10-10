# Toolchain

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| jactionlint | mise (`mise.toml` pins `github:jdx/jactionlint` 2.0.2) | Lint this repo's workflows (`lint.yml`, the pre-push hook and `mise x -- jactionlint`) | https://github.com/jdx/jactionlint |
| hk | mise (`mise.toml` pins 2.5.0) | Runs jactionlint as a pre-push hook (`hk.pkl`) | https://github.com/jdx/hk |
| shellcheck | mise (`mise.toml` pins 0.11.0) | Checks `run:` scripts inside jactionlint | https://github.com/koalaman/shellcheck |
| git | system package | Version control | https://git-scm.com |
| rustc | the calling repo's mise pin, inside `ci-rust.yml` | The reusable workflow checks that `rustc --version` matches the pin | https://github.com/rust-lang/rust |

GitHub Actions runs the workflows. This repo does not publish a library.
The only Cargo package is the self-test fixture `tests/fixtures/rust-crate`, which has no dependencies.
