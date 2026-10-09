# Использование pyrlyn/ci

## ci-rust.yml

Две задачи:

- `fmt`: один раз `cargo fmt --all --check`.
- `rust`: по одной задаче на каждую запись общей матрицы целей, `fail-fast: true`. Для своей цели каждая запускает `cargo clippy --all-targets --all-features -- -D warnings` и `cargo check --all-targets`, затем тесты (`test-command`), если раннер может выполнить цель нативно, либо сборку каждой цели (`build-command`), если не может.

Общая матрица (значение по умолчанию для входа `matrix`):

| Раннер | Цель | Тесты |
| --- | --- | --- |
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` | запускаются |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | запускаются |
| `xcode-27` | `aarch64-apple-darwin` | запускаются |
| `windows-latest` | `x86_64-pc-windows-msvc` | запускаются |

macOS только arm64 (Apple Silicon). Эта запись идёт на образе GitHub `xcode-27` (macOS с Xcode 27; `macos-latest` и `macos-26` несут Xcode 26.x) и выбирает Xcode 27 через действие `setup-xcode` (вход `xcode-version`, по умолчанию `27`).

Вызывающие репозитории наследуют матрицу: своей матрицы ОС в каждом репозитории нет. Поскольку вызывающий фиксирует SHA коммита, изменение матрицы доходит до репозитория, когда сдвигается его пин. Передавайте `matrix` только чтобы убрать цель, которую репозиторий не поддерживает.

Закреплённый Rust (из `mise.toml` или `rust-version`) экспортируется как `RUSTUP_TOOLCHAIN`, и каждая задача падает, если `rustc --version` ему не соответствует, так что собственный stable образа раннера не подменяет пин.

```yaml
# .github/workflows/ci.yml в вызывающем репозитории
name: ci

on:
  pull_request:
    types: [opened, synchronize, reopened, ready_for_review]
  workflow_dispatch:

permissions:
  contents: read

jobs:
  rust:
    uses: pyrlyn/ci/.github/workflows/ci-rust.yml@<full commit sha>
    with:
      # всё необязательно
      # working-directory: .
      # rust-version: "1.98.1"          # по умолчанию: версия из mise.toml
      # mise-install-args: rust just    # инструменты, которые ставит mise (по умолчанию: rust)
      # clippy-args: --workspace        # дополнительные аргументы clippy/check до `--`
      # tools: nextest                  # инструменты taiki-e/install-action
      # test-command: cargo nextest run --all-targets --locked
      # cache-all-refs: true            # сохранять кэш Cargo на каждой ссылке
```

### Входы

| Вход | По умолчанию | Описание |
| --- | --- | --- |
| `matrix` | общая матрица выше | JSON-массив записей `{"os", "target"}`, необязательно с `"test": false`. |
| `rust-version` | `""` | Точная цепочка через rustup. Пустое значение ставит Rust из `mise.toml` вызывающего через `jdx/mise-action` (соглашение pyrlyn: `mise.toml` — единственный источник версии Rust). |
| `fmt-runs-on` | `ubuntu-latest` | Метка раннера задачи `fmt`. |
| `working-directory` | `.` | Каталог Cargo workspace. |
| `mise-install-args` | `rust` | Инструменты, которые mise ставит, когда `rust-version` пуст. |
| `clippy-args` | `""` | Дополнительные аргументы для clippy и check, до `--`. |
| `tools` | `""` | Инструменты, устанавливаемые через `taiki-e/install-action` (например, `nextest`). |
| `setup-command` | `""` | Bash в каждой задаче матрицы до clippy (заданы `RUNNER_OS` и `TARGET`). |
| `test-command` | `cargo test --all-targets --all-features` | Bash, который запускает тесты на нативных записях. Пустое значение их пропускает. |
| `build-command` | `cargo build --all-targets --all-features --target "$TARGET"` | Bash, который собирает кросс-цель. |
| `cache-all-refs` | `false` | Сохранять кэш Cargo на каждой ссылке. По умолчанию сохраняет только ветка по умолчанию; включите, если CI идёт только на pull request. |

В `mise.toml` вызывающего должен быть закреплён Rust, например:

```toml
[tools]
rust = { version = "1.98.1", components = "rustfmt,clippy" }
```

## Закрепление

Закрепляйте вызывающих на полном 40-символьном SHA коммита (`@<sha>`, с комментарием `# vX.Y.Z`, когда появятся теги), чтобы изменения здесь не доходили до вызывающих без предупреждения.

`self-test.yml` запускает `ci-rust.yml` на `tests/fixtures/rust-crate` в каждом pull request: с общей матрицей, по пути `rust-version` и с одноэлементной пользовательской матрицей (`"test": false`), которая переопределяет `tools`, `setup-command`, `test-command`, `build-command`, `clippy-args` и `cache-all-refs`.

## Секреты

`ci-rust.yml` секреты не нужны. Переиспользуемый workflow не видит секреты вызывающего, пока их не передадут явно или через `secrets: inherit`:

```yaml
jobs:
  rust:
    uses: pyrlyn/ci/.github/workflows/ci-rust.yml@<full commit sha>
    secrets: inherit
```

Добавляйте `secrets: inherit` только для workflow, которым секреты действительно нужны.

## Права

Workflow объявляют `permissions: contents: read`. Вызывающий может только сохранить или сузить права токена; вызываемый workflow не может их повысить.

## Политика Actions вызывающих репозиториев

Этот репозиторий публичный, поэтому любой репозиторий может вызывать его workflow. Политика Actions вызывающего репозитория всё равно применяется к каждому действию внутри вызываемого workflow. При политике «только действия, принадлежащие listepo» сам `pyrlyn/ci` разрешён, но сторонние действия, которые он использует (`actions/checkout`, `jdx/mise-action`, `Swatinem/rust-cache`, `taiki-e/install-action`), заблокированы. Такому репозиторию нужно «Allow actions created by GitHub» и разрешённые шаблоны:

```text
jdx/mise-action@*
Swatinem/rust-cache@*
taiki-e/install-action@*
```
