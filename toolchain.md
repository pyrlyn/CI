# Toolchain

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| actionlint | mise (`mise.toml` pins 1.7.12) | Lint this repo's workflows (`lint.yml` and `mise x -- actionlint`) | https://github.com/rhysd/actionlint |
| git | system package | Version control | https://git-scm.com |
| rustc | the calling repo's mise pin, inside `ci-rust.yml` | The reusable workflow checks that `rustc --version` matches the pin | https://github.com/rust-lang/rust |

GitHub Actions runs the workflows. This repo does not publish a library.
The only Cargo package is the self-test fixture `tests/fixtures/rust-crate`, which has no dependencies.
