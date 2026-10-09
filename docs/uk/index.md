# Використання pyrlyn/ci

## ci-rust.yml

Дві задачі:

- `fmt`: один раз `cargo fmt --all --check`.
- `rust`: по одній задачі на кожен запис спільної матриці цілей, `fail-fast: true`. Для своєї цілі кожна запускає `cargo clippy --all-targets --all-features -- -D warnings` і `cargo check --all-targets`, далі тести (`test-command`), якщо раннер може виконати ціль нативно, або збірку кожної цілі (`build-command`), якщо не може.

Спільна матриця (значення за замовчуванням для входу `matrix`):

| Раннер | Ціль | Тести |
| --- | --- | --- |
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` | запускаються |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | запускаються |
| `xcode-27` | `aarch64-apple-darwin` | запускаються |
| `windows-latest` | `x86_64-pc-windows-msvc` | запускаються |

macOS лише arm64 (Apple Silicon). Цей запис іде на образі GitHub `xcode-27` (macOS з Xcode 27; `macos-latest` і `macos-26` несуть Xcode 26.x) і вибирає Xcode 27 через дію `setup-xcode` (вхід `xcode-version`, за замовчуванням `27`).

Репозиторії, що викликають, успадковують матрицю: власної матриці ОС у кожному репозиторії немає. Оскільки викликач фіксує SHA коміту, зміна матриці доходить до репозиторію, коли зсувається його пін. Передавайте `matrix` лише щоб прибрати ціль, яку репозиторій не підтримує.

Закріплений Rust (з `mise.toml` або `rust-version`) експортується як `RUSTUP_TOOLCHAIN`, і кожна задача падає, якщо `rustc --version` йому не відповідає, тож власний stable образу раннера не підміняє пін.

```yaml
# .github/workflows/ci.yml у репозиторії, що викликає
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
      # усе необов'язкове
      # working-directory: .
      # rust-version: "1.98.1"          # за замовчуванням: версія з mise.toml
      # mise-install-args: rust just    # інструменти, які ставить mise (за замовчуванням: rust)
      # clippy-args: --workspace        # додаткові аргументи clippy/check до `--`
      # tools: nextest                  # інструменти taiki-e/install-action
      # test-command: cargo nextest run --all-targets --locked
      # cache-all-refs: true            # зберігати кеш Cargo на кожному посиланні
```

### Входи

| Вхід | За замовчуванням | Опис |
| --- | --- | --- |
| `matrix` | спільна матриця вище | JSON-масив записів `{"os", "target"}`, необов'язково з `"test": false`. |
| `rust-version` | `""` | Точний ланцюжок через rustup. Порожнє значення ставить Rust з `mise.toml` викликача через `jdx/mise-action` (угода pyrlyn: `mise.toml` — єдине джерело версії Rust). |
| `fmt-runs-on` | `ubuntu-latest` | Мітка раннера задачі `fmt`. |
| `working-directory` | `.` | Каталог Cargo workspace. |
| `mise-install-args` | `rust` | Інструменти, які mise ставить, коли `rust-version` порожній. |
| `clippy-args` | `""` | Додаткові аргументи для clippy і check, до `--`. |
| `tools` | `""` | Інструменти, що встановлюються через `taiki-e/install-action` (наприклад, `nextest`). |
| `setup-command` | `""` | Bash у кожній задачі матриці до clippy (задані `RUNNER_OS` і `TARGET`). |
| `test-command` | `cargo test --all-targets --all-features` | Bash, який запускає тести на нативних записах. Порожнє значення їх пропускає. |
| `build-command` | `cargo build --all-targets --all-features --target "$TARGET"` | Bash, який збирає крос-ціль. |
| `cache-all-refs` | `false` | Зберігати кеш Cargo на кожному посиланні. За замовчуванням зберігає лише гілка за замовчуванням; увімкніть, якщо CI іде лише на pull request. |

У `mise.toml` викликача має бути закріплений Rust, наприклад:

```toml
[tools]
rust = { version = "1.98.1", components = "rustfmt,clippy" }
```

## Закріплення

Закріплюйте викликачів на повному 40-символьному SHA коміту (`@<sha>`, з коментарем `# vX.Y.Z`, коли з'являться теги), щоб зміни тут не доходили до викликачів без попередження.

`self-test.yml` запускає `ci-rust.yml` на `tests/fixtures/rust-crate` у кожному pull request: зі спільною матрицею, шляхом `rust-version` і з одноелементною користувацькою матрицею (`"test": false`), яка перевизначає `tools`, `setup-command`, `test-command`, `build-command`, `clippy-args` і `cache-all-refs`.

## Секрети

`ci-rust.yml` секрети не потрібні. Багаторазовий workflow не бачить секретів викликача, доки їх не передадуть явно або через `secrets: inherit`:

```yaml
jobs:
  rust:
    uses: pyrlyn/ci/.github/workflows/ci-rust.yml@<full commit sha>
    secrets: inherit
```

Додавайте `secrets: inherit` лише для workflow, яким секрети справді потрібні.

## Права

Workflow оголошують `permissions: contents: read`. Викликач може лише зберегти або звузити права токена; викликаний workflow не може їх підвищити.

## Політика Actions репозиторіїв, що викликають

Цей репозиторій публічний, тому будь-який репозиторій може викликати його workflow. Політика Actions репозиторію, що викликає, все одно застосовується до кожної дії всередині викликаного workflow. За політики «лише дії, що належать listepo» сам `pyrlyn/ci` дозволений, але сторонні дії, які він використовує (`actions/checkout`, `jdx/mise-action`, `Swatinem/rust-cache`, `taiki-e/install-action`), заблоковані. Такому репозиторію потрібно «Allow actions created by GitHub» і дозволені шаблони:

```text
jdx/mise-action@*
Swatinem/rust-cache@*
taiki-e/install-action@*
```
