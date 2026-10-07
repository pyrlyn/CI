# Using pyrlyn/ci

## ci-rust.yml

Two jobs:

- `fmt`: `cargo fmt --all --check`, once.
- `rust`: one job per entry of the shared target matrix, `fail-fast: true`. Each runs
  `cargo clippy --all-targets --all-features -- -D warnings` and `cargo check --all-targets`
  for its target, then the tests (`test-command`) when the runner executes the target
  natively, or a build of every target (`build-command`) when it does not.

The shared matrix (default of the `matrix` input):

| Runner | Target | Tests |
| --- | --- | --- |
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` | run |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | run |
| `xcode-27` | `aarch64-apple-darwin` | run |
| `windows-latest` | `x86_64-pc-windows-msvc` | run |

macOS is arm64 (Apple Silicon) only. Its entry runs on GitHub's `xcode-27` image (macOS with
Xcode 27; `macos-latest` and `macos-26` carry Xcode 26.x) and selects Xcode 27 through the
`setup-xcode` action (`xcode-version` input, default `27`).

Callers inherit it: no per-repository OS matrix. Because callers pin a commit SHA, a matrix
change reaches a repository when its pin moves. Pass `matrix` only to drop a target the
repository does not support.

The pinned Rust (from `mise.toml`, or `rust-version`) is exported as `RUSTUP_TOOLCHAIN`, and
every job fails when `rustc --version` does not match it, so the runner image's own stable
never stands in for the pin.

```yaml
# .github/workflows/ci.yml in the calling repository
name: ci

on:
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

jobs:
  rust:
    uses: pyrlyn/ci/.github/workflows/ci-rust.yml@<full commit sha>
    with:
      # all optional
      # working-directory: .
      # rust-version: "1.98.1"          # default: the version in mise.toml
      # mise-install-args: rust just    # tools mise installs (default: rust)
      # clippy-args: --workspace        # extra clippy/check args before `--`
      # tools: nextest                  # taiki-e/install-action tools
      # test-command: cargo nextest run --all-targets --locked
      # cache-all-refs: true            # save the Cargo cache on every ref
```

### Inputs

| Input | Default | Description |
| --- | --- | --- |
| `matrix` | the shared matrix above | JSON array of `{"os", "target"}` entries, optionally `"test": false`. |
| `rust-version` | `""` | Exact toolchain via rustup. Empty installs Rust from the caller's `mise.toml` with `jdx/mise-action` (pyrlyn convention: `mise.toml` is the single source of the Rust version). |
| `fmt-runs-on` | `ubuntu-latest` | Runner label of the `fmt` job. |
| `working-directory` | `.` | Cargo workspace directory. |
| `mise-install-args` | `rust` | Tools mise installs when `rust-version` is empty. |
| `clippy-args` | `""` | Extra arguments for clippy and check, placed before `--`. |
| `tools` | `""` | Tools installed with `taiki-e/install-action` (e.g. `nextest`). |
| `setup-command` | `""` | Bash run in every matrix job before clippy (`RUNNER_OS` and `TARGET` are set). |
| `test-command` | `cargo test --all-targets --all-features` | Bash that runs the tests on native entries. Empty skips them. |
| `build-command` | `cargo build --all-targets --all-features --target "$TARGET"` | Bash that builds a cross target. |
| `cache-all-refs` | `false` | Save the Cargo cache on every ref. By default only the default branch saves; set this when CI runs only on pull requests. |

The caller's `mise.toml` should pin Rust, for example:

```toml
[tools]
rust = { version = "1.98.1", components = "rustfmt,clippy" }
```

## Pinning

Pin callers to a full 40-character commit SHA (`@<sha>`, with a `# vX.Y.Z` comment once tags
exist) so changes here do not reach callers unannounced.

`self-test.yml` runs `ci-rust.yml` against `tests/fixtures/rust-crate` on every pull
request, with the shared matrix and through the `rust-version` path.

## Secrets

`ci-rust.yml` needs no secrets. A reusable workflow does not see the caller's secrets unless
they are passed explicitly or with `secrets: inherit`:

```yaml
jobs:
  rust:
    uses: pyrlyn/ci/.github/workflows/ci-rust.yml@<full commit sha>
    secrets: inherit
```

Only add `secrets: inherit` for workflows that actually need secrets.

## Permissions

The workflows declare `permissions: contents: read`. A caller can only keep or reduce the
token permissions; the called workflow cannot raise them.

## Actions policy of calling repositories

This repository is public, so any repository can call its workflows. The calling
repository's Actions policy still applies to every action used inside the called workflow.
With "only actions owned by listepo", `pyrlyn/ci` itself is allowed but the third-party
actions it uses (`actions/checkout`, `jdx/mise-action`, `Swatinem/rust-cache`, `taiki-e/install-action`) are blocked.
Such a repository needs "Allow actions created by GitHub" and these patterns allowed:

```text
jdx/mise-action@*
Swatinem/rust-cache@*
taiki-e/install-action@*
```
