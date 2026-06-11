# Plan architektury aplikacji `smol`

**Spis treści**
1. Nazwa i cel
2. Krytyczna analiza: CLI vs dedykowana powłoka
3. Architektura ogólna
4. Tryby działania
5. System konfiguracji parserów (TOML)
6. System zadań (Task System)
7. Lista wspieranych komend
8. Plan TDD i struktura testów
9. Struktura plików/katalogów
10. mock-command — narzędzie testowe
11. Krytyczna ocena planu

---

## 1. Nazwa i cel

**Nazwa**: `smol` (skrótowiec: *Smart Minimal Output Logger*)

**Cel**: Aplikacja pośrednicząca między LLM a terminalem. Uruchamia dowolną komendę, przechwytuje jej output, analizuje go w kontekście znanego narzędzia (lub heurystycznie), zwraca **krótkie podsumowanie** (np. `success` / `errors: 5 warnings: 3 task-id:a3f9`). Długotrwałe zadania może uruchamiać w tle. Raw logi są zawsze zapisywane do pliku, a LLM może je później pobrać po `task-id`.

---

## 2. Krytyczna analiza: CLI vs dedykowana powłoka

### Opcja A: `smol <command> [args...]`

```
smol mvn clean install
smol cargo build --release
smol --mode auto -- mvn clean install
```

**Zalety:**
- Kompozycyjność z istniejącym shell: `smol cargo build && echo "done"`
- Znajomość: każdy programista wie jak to działa
- Prosta implementacja: `std::process::Command` + pipe'y
- Łatwe parsowanie argumentów (clap/bpaf)
- Nie wymaga implementacji historii, autouzupełniania, job control, pipe'ów, redirectów
- Można łączyć: `smol --mode bg make -j4 && smol status last`

**Wady:**
- Potoki i przekierowania muszą być w cudzysłowie: `smol "cargo build | head -5"` — inaczej shell zewnętrzny przechwyci pipe przed smol
- Konflikt flag: `smol ls -la` — `-la` może być interpretowane przez smol lub przez ls; wymaga separatora `--`
- Mniej "immersyjny" dla LLM-a (LLM musi pamiętać o prefiksie `smol`)

### Opcja B: Dedykowana powłoka (REPL)

```
smol
smol> mvn clean install
smol> cargo build
```

**Zalety:**
- Pełna kontrola nad potokami i redirectami wewnątrz smol
- Naturalny dla LLM-a (raz wchodzi w shell i już wie)
- Można dodać kontekst LLM-aware: `set mode auto`

**Wady:**
- **Ogromna złożoność**: trzeba zaimplementować parsowanie składni shella (pipe'y, `&&`, `||`, `;`, redirecty, zmienne środowiskowe, globbing, quoting, escaping)
- Niekompatybilność: userzy muszą uczyć się nowego shella, a nie będzie w 100% bash-compatible
- Utrudnione łączenie z Unix tools: nie można prosto zrobić `smol mvn install | grep ERROR`
- Większy koszt utrzymania
- Dla LLM-a to szczegół implementacyjny — i tak wywołuje komendy jako stringi

### Werdykt

**Wybieram opcję A** (`smol <command>`). Jest prostsza, bardziej uniksowa i stanowi mniejsze ryzyko implementacyjne. Dla problemu z pipe'ami wystarczy dokumentacja: `smol "cargo build | head -20"` uruchomi całość w shellu (przez `sh -c`). Dla flag używamy `--` jako separatora: `smol -- ls -la`.

---

## 3. Architektura ogólna

```
┌──────────────────────────────────────────────────┐
│                    CLI (clap/bpaf)                │
│  smol [opcje] <komenda> [args...]                │
│  smol status <task-id>                            │
│  smol log <task-id>                               │
│  smol list                                        │
│  smol cancel <task-id>                            │
└──────────┬───────────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────────────┐
│              Command Executor                     │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ Spawner  │  │ Watcher  │  │ Backgrounder  │  │
│  │ (uruchom)│  │ (czeka)  │  │ (detach/fork) │  │
│  └──────────┘  └──────────┘  └───────────────┘  │
└──────────┬───────────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────────────┐
│           Output Pipeline                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ Capture  │  │ Parser   │  │ Summarizer    │  │
│  │ (stdout  │  │ (error/  │  │ (statystyki,  │  │
│  │  +stderr)│  │  warning)│  │  pierwsze N)  │  │
│  └──────────┘  └──────────┘  └───────────────┘  │
└──────────┬───────────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────────────┐
│              Task Storage                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ Registry │  │ Log Files│  │ Metadata      │  │
│  │ (SQLite  │  │ (raw     │  │ (JSON/TOML)   │  │
│  │  lub TOML)│  │  output) │  │               │  │
│  └──────────┘  └──────────┘  └───────────────┘  │
└──────────────────────────────────────────────────┘
```

### Główne moduły:

| Moduł | Opis |
|-------|------|
| **`smol-cli`** | Parsowanie argumentów (bpaf), routing komend |
| **`smol-exec`** | Uruchamianie procesów, async I/O, timeout, background |
| **`smol-parse`** | System parserów + heurystyki; wykrywanie błędów/warningów |
| **`smol-config`** | Wczytywanie konfiguracji TOML dla narzędzi |
| **`smol-storage`** | Task registry, zapis/odczyt logów i metadanych |
| **`smol-mock`** | `mock-command` do testów (deterministyczny output) |
| **`smol-core`** | Typy wspólne: TaskId, TaskStatus, Summary, ParserConfig |

---

## 4. Tryby działania

### `--mode sync` (domyślny lub jawny)

```
smol --mode sync cargo build
```

- Blokuje do zakończenia komendy
- Zbiera cały output
- Parsuje i zwraca podsumowanie
- Zapisuje raw log do pliku
- Exit code = 0 jeśli sukces, 1 jeśli błędy

### `--mode auto` (zalecany domyślny)

```
smol --mode auto mvn clean install
```

- Uruchamia komendę i zaczyna czytać output
- Ustala **timeout** (np. 5 sekund, konfigurowalny: `--wait 10`)
- **Jeśli komenda zakończy się przed timeoutem**: zachowuje się jak `sync`
- **Jeśli komenda wciąż działa po timeoutcie**: 
  1. Przydziela `task-id` (8 znaków base62)
  2. Zapisuje partial output do pliku
  3. Detachuje proces (fork + setsid)
  4. Zwraca: `task-id:a3f9k2 — command is running in background`
  5. Dalszy output jest appendowany do pliku przez proces potomny

### `--mode bg` (background)

```
smol --mode bg make -j8
```

- Natychmiast uruchamia w tle
- Zwraca `task-id:a3f9k2`
- Nie czeka ani sekundy

### Komendy operacyjne na taskach

| Komenda | Opis |
|---------|------|
| `smol status <task-id>` | Status: running/done/failed, statystyki |
| `smol status --last` | Status ostatniego taska |
| `smol log <task-id>` | Raw output (do paginacji przez `less`/`head`) |
| `smol log <task-id> --errors` | Tylko linie z błędami |
| `smol log <task-id> --warnings` | Tylko warningi |
| `smol log <task-id> --stats` | JSON z metadanymi i statystykami |
| `smol list` | Lista wszystkich tasków |
| `smol list --running` | Tylko uruchomione |
| `smol cancel <task-id>` | Wyślij SIGTERM do procesu |
| `smol clean --older 24h` | Usuń zakończone taski starsze niż N |

### Użycie task-id przez LLM

```
$ smol --mode auto mvn clean install
[wait 5s... still running]
task-id:a3f9k2

$ smol status a3f9k2
Status: completed (0.0.35s ago)
Duration: 2m 14s
Errors: 5   Warnings: 23
First 3 errors:
  - src/main/java/App.java:42: cannot find symbol
  - src/main/java/App.java:55: incompatible types
  - src/main/java/Config.java:12: unreachable statement

$ smol log a3f9k2 --errors | head -20
[..pierwsze 20 linii błędów..]

$ smol log a3f9k2 --stats
{
  "command": "mvn clean install",
  "exit_code": 1,
  "duration_sec": 134,
  "total_lines": 2843,
  "errors": 5,
  "warnings": 23,
  "error_lines": [47, 83, 91, 142, 188],
  "warning_lines": [12, 15, 33, ...],
  "status": "failed"
}
```

---

## 5. System konfiguracji parserów (TOML)

### Lokalizacja

Pliki konfiguracyjne dla parserów znajdują się w:
```
~/.smol/parsers/
  maven.toml
  cargo.toml
  gcc.toml
  generic.toml        ← fallback
  docker.toml
  git.toml
```

Kolejność ładowania:
1. Wbudowane parserki (spakowane w binary)
2. `~/.smol/parsers/*.toml` (nadpisują / dodają nowe)
3. `./.smol/parsers/*.toml` (per-project override)

### Schemat TOML

```toml
# ~/.smol/parsers/maven.toml

[name]
# Jak wykryć to narzędzie?
detection = { command_prefix = "mvn" }  # po nazwie komendy
# detection = { heuristic = { regex = "\\[INFO\\] .*", min_lines = 3 } }  # lub po treści outputu

# Priorytet: command_prefix > heuristic
# Jeśli żaden nie matchuje → generic fallback

[parsing]
# Linie ignorowane (debug, info noise)
ignore_patterns = [
  "^\\[INFO\\] ---",
  "^\\[INFO\\] Scanning",
  "^\\[INFO\\] Reactor",
  "^Downloading from",
  "^Progress",
]

# Detekcja błędów
error_patterns = [
  { regex = "^(.*?):(\\d+):(\\d+):\\s+error:\\s+(.*)$", severity = "error" },
  { regex = "^\\[ERROR\\]\\s+(.*)$", severity = "error" },
  { regex = "^\\[FATAL\\]\\s+(.*)$", severity = "error" },
  { regex = "BUILD FAILURE", severity = "error", is_fatal = true },
]

# Detekcja warningów
warning_patterns = [
  { regex = "^(.*?):(\\d+):(\\d+):\\s+warning:\\s+(.*)$", severity = "warning" },
  { regex = "^\\[WARNING\\]\\s+(.*)$", severity = "warning" },
]

# Detekcja końcowego statusu
status_patterns = {
  success = [
    { regex = "^\\[INFO\\] BUILD SUCCESS", group = "message" },
    { regex = "BUILD SUCCESSFUL", group = "message" },
  ],
  failure = [
    { regex = "^\\[INFO\\] BUILD FAILURE", group = "message" },
    { regex = "^\\[ERROR\\] BUILD FAIL", group = "message" },
  ]
}

# Dla podsumowania: ile błędów/warningów pokazać domyślnie
summary = { max_errors = 3, max_warnings = 5, show_error_lines = true }
```

### Heurystyczny fallback (generic.toml)

Dla nieznanych komend działa parser generyczny, który szuka:

- Wzorców typu `filename:line:column: error:` (GCC/Clang-style)
- Wzorców `ERROR | Error | error | FAIL | failed`
- Wzorców `WARNING | Warning | warning`
- Wzorców `^Traceback (most recent call last)` (Python)
- Linii z `exit code N` / `Exit code N`
- Wzorców `^\[.*\]` (log4j/logback/logging patterns)
- Końcowe "Process finished with exit code X"

### Proces detekcji narzędzia

```
1. Weź nazwę komendy (np. "mvn")
2. Sprawdź command_prefix w parserach (prefix match "mvn" → MavenParser)
3. Jeśli nie znaleziono → rozpocznij output capture
4. Po N liniach (konfigurowalne, domyślnie 10) uruchom wszystkie heurystyki
5. Jeśli heurystyka matchuje z confidence > threshold → użyj tego parsera
6. Jeśli nadal nie → użyj generic fallback
```

---

## 6. System zadań (Task System)

### Task ID

- 8 znaków w base62 (0-9, a-z, A-Z) = ~218 bilionów kombinacji
- Generowane z `/dev/urandom` lub z hash(time + PID + counter)
- Krótkie, wygodne do kopiowania i wklejania

### Przechowywanie

```
~/.smol/
├── smol.toml              ← konfiguracja globalna
├── parsers/               ← konfiguracje parserów
│   ├── maven.toml
│   ├── cargo.toml
│   └── ...
├── tasks/
│   ├── registry.toml      ← indeks wszystkich tasków
│   ├── a3f9k2/
│   │   ├── meta.toml      ← metadane (komenda, czas, status, statystyki)
│   │   ├── output.log     ← raw stdout
│   │   └── error.log      ← raw stderr
│   └── b8xR1a/
│       ├── meta.toml
│       ├── output.log
│       └── error.log
└── tasks.db.sqlite        ← alternatywnie: SQLite zamiast registry.toml
```

### Struktura meta.toml

```toml
[task]
id = "a3f9k2"
command = "mvn clean install"
mode = "auto"
created_at = "2026-06-11T14:23:11Z"
completed_at = "2026-06-11T14:25:25Z"
exit_code = 1
duration_sec = 134
status = "failed"  # running | success | failed | cancelled | timeout

[stats]
total_lines = 2843
total_chars = 194532
output_size_bytes = 194532
error_count = 5
warning_count = 23
error_lines = [47, 83, 91, 142, 188]
warning_lines = [12, 15, 33, 55, 77, 92, 105, 128, 144, 160, ...]

[errors]
# Pierwsze N błędów (konfigurowalne)
lines = [
  { line = 47, content = "App.java:42: error: cannot find symbol", file = "App.java", file_line = 42, column = 5 },
  { line = 83, content = "App.java:55: error: incompatible types", file = "App.java", file_line = 55 },
  ...
]

[warnings]
lines = [ ... ]

[process]
pid = 48291
background_pid = 48302  # jeśli był fork dla bg
```

---

## 7. Lista wspieranych komend

### Kategoria 1 — **Budowanie i kompilacja** (najważniejsze — długi output, dużo błędów/warningów)

| Komenda | Narzędzie | Output profile | Obsługa w MVP |
|---------|-----------|----------------|---------------|
| `mvn` | Apache Maven | Bardzo długi (INFO), errors/warnings w [ERROR] [WARNING] | **Tak** |
| `./mvnw` | Maven Wrapper | jw. | **Tak** (alias na maven) |
| `gradle` / `./gradlew` | Gradle | Długi, errors/warnings | Tak |
| `cargo` | Cargo (Rust) | Średni, errors z rustc | **Tak** |
| `rustc` | Rust compiler | Krótki/średni, specyficzne errory | Tak |
| `make` | GNU Make | Zależy od podkomend | **Tak** |
| `cmake` | CMake | Średni (generuje Makefile) | Tak |
| `ninja` | Ninja build | Krótki, errors jak gcc | Tak |
| `gcc` / `g++` / `cc` | GNU C/C++ | Klasyczne `file:line:col: error:` | **Tak** |
| `clang` | LLVM Clang | jw. | **Tak** (wzór jak gcc) |
| `cl.exe` | MSVC | Inny format: `file(line) : error` | Opcjonalnie |
| `go` | Go (build/test) | Zwięzły, errors: `file:line: error` | Tak |
| `tsc` | TypeScript | Długi, errors/warnings | Tak |
| `esbuild` | esbuild | Zwięzły | Opcjonalnie |
| `dotnet` | .NET CLI | Długi, errors/warnings | Tak |
| `bazel` | Bazel | Bardzo długi | Opcjonalnie |
| `sbt` | Scala Build Tool | Długi | Opcjonalnie |
| `nvcc` | NVIDIA CUDA | errors/warnings | Opcjonalnie |
| `swift` / `swift build` | Swift | errors/warnings | Opcjonalnie |

### Kategoria 2 — **Task runnery**

| Komenda | Opis | Uwagi |
|---------|------|-------|
| `task` | Task runner (Go) | Output zależy od tasków; parser generic |
| `just` | Just command runner | jw. |
| `make` | (jw.) | Często używany jako task runner |
| `npm run` / `npx` | npm scripts | Output zależy od skryptu |
| `yarn` / `pnpm` | Alternatywy npm | jw. |
| `poetry run` | Python | jw. |

### Kategoria 3 — **DevOps i Docker**

| Komenda | Output profile |
|---------|---------------|
| `docker build` | **Bardzo długi** (warstwy, cache, errory) |
| `docker pull` | Umiarkowany (progress bari) |
| `docker compose up` | Długi (logi z kontenerów) |
| `kubectl apply` | Krótki |
| `kubectl logs` | **Potencjalnie bardzo długi** |
| `terraform plan` / `apply` | Średni/długi, zmiany + errory |
| `helm install` / `upgrade` | Średni |
| `ansible-playbook` | Długi (taski + errory) |
| `vagrant up` | Długi |

### Kategoria 4 — **Wyszukiwanie i przetwarzanie** (potencjalnie ogromny output)

| Komenda | Ryzyko outputu | Rekomendacja w smol |
|---------|----------------|---------------------|
| `grep -r` pattern src/ | **Ogromny** | Auto-truncation + stats |
| `rg` / `ripgrep` pattern | **Ogromny** | Auto-truncation + stats |
| `find . -name '*.java'` | Duży | Auto-truncation |
| `ls -laR` | **Ogromny** | Auto-truncation |
| `tree` | Duży | Auto-truncation |
| `cat` (duży plik) | **Ogromny** | Auto-truncation |
| `diff` / `git diff` | Duży | Truncation + stats |
| `git log --oneline -1000` | Duży | Truncation |

### Kategoria 5 — **Lintery i analiza kodu**

| Komenda | Output profile |
|---------|---------------|
| `clippy` (przez cargo) | errors/warnings/suggestions |
| `clang-tidy` | Długi, dużo warningów |
| `ruff` | errors/warnings |
| `flake8` | errors/warnings |
| `pylint` | Długi, dużo warningów |
| `eslint` | errors/warnings |
| `shellcheck` | errors/warnings |

### Kategoria 6 — **Narzędzia developerskie**

| Komenda | Uwagi |
|---------|-------|
| `git status` / `diff` / `log` | Częste w AI workflows |
| `gh` (GitHub CLI) | Różny output |
| `curl` + URL | Duże odpowiedzi → truncation |
| `wget` | jw. |
| `ssh host command` | Zdalny output |
| `ps aux` / `top -b -n1` | Duży, ale strukturalny |
| `df -h` / `du -sh *` | Mały/średni |
| `lsof` | Duży |
| `journalctl -u service` | **Bardzo długi** |
| `dmesg` | Średni |

### Strategia dla MVP

**Priorytet P0** (własne parsery TOML + testy):
- `mvn` / `mvnw`
- `cargo` (w tym `cargo build`, `cargo test`, `cargo clippy`)
- `gcc` / `clang` / `g++` (C/C++ compilers)
- `make`
- `docker` (głównie `docker build`)
- `git` (status, diff, log)

**Priorytet P1** (generyczne parsery + testy):
- `go build` / `go test`
- `tsc`
- `npm` / `npx`
- `gradle`
- `cmake`
- `rustc`

**Priorytet P2** (tylko generic fallback):
- Reszta — działa przez `generic.toml` z heurystykami

---

## 8. Plan TDD i struktura testów

### Filozofia

- **Red-Green-Refactor** dla każdej funkcjonalności
- Testy jednostkowe: każdy moduł w izolacji
- Testy integracyjne: `mock-command` jako zamiennik realnych narzędzi
- Testy end-to-end: `smol mock-command ...` przez CLI
- Determinizm: mocki zamiast prawdziwych kompilatorów
- Szybkość: testy nie mogą wymagać internetu ani długich buildów

### Struktura testów

```
tests/
├── e2e/
│   ├── cli_test.rs              ← testy interfejsu CLI
│   ├── modes_test.rs            ← sync/auto/bg modes
│   └── task_commands_test.rs    ← status/log/list/cancel
├── integration/
│   ├── parser_detection_test.rs ← detekcja narzędzia po komendzie i heurystykach
│   ├── maven_parser_test.rs     ← Maven: output mock + parsing
│   ├── cargo_parser_test.rs     ← Cargo: output mock + parsing
│   ├── gcc_parser_test.rs       ← GCC: output mock + parsing
│   ├── generic_parser_test.rs   ← Fallback + heurystyki
│   └── output_capture_test.rs   ← capture stdout/stderr, truncation
├── unit/
│   ├── task_id_test.rs          ← generowanie ID, unikalność
│   ├── config_test.rs           ← ładowanie TOML, walidacja
│   ├── storage_test.rs          ← zapis/odczyt meta.toml, log files
│   ├── summarizer_test.rs       ← generowanie podsumowania
│   └── heuristic_test.rs        ← regex matchowanie
└── fixtures/
    ├── outputs/
    │   ├── maven_success.txt
    │   ├── maven_failure.txt
    │   ├── cargo_success.txt
    │   ├── cargo_warnings.txt
    │   ├── cargo_errors.txt
    │   ├── gcc_errors.txt
    │   ├── generic_no_match.txt
    │   └── mixed_output.txt
    ├── configs/
    │   ├── maven.toml
    │   ├── cargo.toml
    │   └── custom.toml
    └── mock_commands/
        ├── maven_success.sh      ← symuluje output mavena (deterministyczny)
        ├── maven_failure.sh
        └── cargo_build.sh
```

### mock-command

**Specjalna komenda wbudowana w smol** (a nie osobny binary):

```
smol mock-command --name maven_success
smol mock-command --name cargo_errors --error-count 5
smol mock-command --name gcc_errors --file test.c --line 42
smol mock-command --name slow --delay 30 --name cargo_success
smol mock-command --name generic --lines 100
smol mock-command --name maven_failure --stream stderr
smol mock-command --name custom --file testdata/output.txt
```

Działanie: projektanci definiują w `tests/fixtures/outputs/` pliki z gotowym outputem. `mock-command` czyta je i wypisuje na stdout/stderr z opcjonalnym opóźnieniem.

**Implementacja**: osobny moduł `smol-mock`, który w trybie `--mock` rejestruje komendę `mock-command` w CLI.

### Przebieg testów

```
# Test 1: basic success
$ smol mock-command --name maven_success
> success

# Test 2: errors detected
$ smol mock-command --name maven_failure
> errors: 5 warnings: 23 task-id:a3f9k2

# Test 3: background mode
$ smol --mode bg mock-command --name slow --delay 30 --name cargo_success
> task-id:xyz123

# Test 4: auto mode (fast command)
$ smol --mode auto mock-command --name maven_success
> success

# Test 5: auto mode (slow command — transitions to background)
$ smol --mode auto mock-command --name slow --delay 10 --name maven_success
> [wait 5s...] task-id:abc456
```

---

## 9. Struktura plików/katalogów

```
smol/
├── Cargo.toml
├── Cargo.lock
├── PLAN.md                    ← niniejszy dokument
├── README.md
├── smol.toml                  ← domyślna konfiguracja (w projekcie)
├── parsers/                   ← domyślne konfiguracje parserów
│   ├── maven.toml
│   ├── cargo.toml
│   ├── gcc.toml
│   ├── docker.toml
│   ├── git.toml
│   ├── go.toml
│   ├── tsc.toml
│   ├── gradle.toml
│   ├── npm.toml
│   └── generic.toml
├── src/
│   ├── main.rs                ← entry point
│   ├── lib.rs                 ← re-exporty
│   ├── cli.rs                 ← definicje CLI (bpaf)
│   ├── app.rs                 ← główna logika aplikacji
│   │
│   ├── exec/                  ← moduł exec
│   │   ├── mod.rs
│   │   ├── spawner.rs         ← uruchamianie procesu
│   │   ├── watcher.rs         ← obserwacja + timeout (async)
│   │   ├── backgrounder.rs    ← detach + fork dla bg
│   │   └── signal.rs          ← SIGTERM/SIGKILL dla cancel
│   │
│   ├── parse/                 ← moduł parserów
│   │   ├── mod.rs
│   │   ├── detector.rs        ← detekcja narzędzia po nazwie / heurystycznie
│   │   ├── engine.rs          ← uruchomienie właściwego parsera
│   │   ├── matcher.rs         ← regex matchowanie z konfiguracji
│   │   ├── generic.rs         ← fallback parser
│   │   ├── summarizer.rs      ← generowanie podsumowania
│   │   └── parsers/           ← zarejestrowane parsery (opcjonalnie)
│   │       ├── mod.rs
│   │       ├── maven.rs
│   │       ├── cargo.rs
│   │       └── gcc.rs
│   │
│   ├── config/                ← moduł konfiguracji
│   │   ├── mod.rs
│   │   ├── loader.rs          ← wczytywanie z katalogów
│   │   ├── parser_config.rs   ← struktury TOML
│   │   ├── global_config.rs   ← smol.toml (ustawienia globalne)
│   │   └── validation.rs      ← walidacja configu
│   │
│   ├── storage/               ← moduł przechowywania
│   │   ├── mod.rs
│   │   ├── registry.rs        ← registry.toml / SQLite
│   │   ├── task_store.rs      ← zapis/odczyt pojedynczego taska
│   │   ├── cleanup.rs         ← automatyczne czyszczenie starych tasków
│   │   └── paths.rs           ← ścieżki (~/.smol/...)
│   │
│   ├── mock/                  ← moduł mock-command
│   │   ├── mod.rs
│   │   └── mock_command.rs    ← implementacja mock-command
│   │
│   └── core/                  ← typy wspólne
│       ├── mod.rs
│       ├── task_id.rs         ← TaskId (newtype)
│       ├── task.rs            ← Task, TaskStatus
│       ├── summary.rs         ← Summary, ErrorLine, WarningLine
│       ├── parser_config.rs   ← ParserConfig, PatternConfig
│       └── error.rs           ← błędy aplikacji (thiserror)
│
├── tests/
│   ├── e2e/
│   │   ├── cli_test.rs
│   │   ├── modes_test.rs
│   │   └── ...
│   ├── integration/
│   │   ├── parser_detection_test.rs
│   │   ├── ...
│   └── fixtures/
│       ├── outputs/
│       └── configs/
│
└── benches/                   ← benchmarki (opcjonalnie)
    └── parser_bench.rs
```

### Kluczowe zależności w Cargo.toml

```toml
[dependencies]
bpaf = { version = "0.9", features = ["derive"] }  # CLI argument parsing
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"                 # parsing TOML config
regex = "1"                  # regex patterns z parser config
uuid = { version = "1", features = ["v4"] }   # lub custom base62 ID
chrono = "0.4"               # timestampy
thiserror = "2"              # error handling
tracing = "0.1"              # logowanie (debugowanie smol-a)
tracing-subscriber = "0.3"

[dev-dependencies]
assert_cmd = "2"             # testowanie CLI
predicates = "3"             # asercje na output
tempfile = "3"               # tymczasowe katalogi testowe
insta = "1"                  # snapshot testing (opcjonalnie)
```

---

## 10. mock-command — szczegóły implementacji

### Rejestracja w CLI

```rust
// W cli.rs dodajemy podkomendę "mock-command"
// Działa tylko w dev mode (cfg(debug_assertions)) lub po fladze --enable-mock
enum Subcommand {
    Run {
        command: Vec<String>,
        #[bpaf(external)]
        mode: Mode,
    },
    Status {
        task_id: String,
    },
    Log {
        task_id: String,
        #[bpaf(long, short)]
        errors: bool,
        #[bpaf(long, short)]
        warnings: bool,
        #[bpaf(long, short)]
        stats: bool,
    },
    Mock {
        #[bpaf(long, short)]
        name: String,
        #[bpaf(long, short)]
        delay: Option<f64>,
        #[bpaf(long, short)]
        error_count: Option<usize>,
        #[bpaf(long, short)]
        warning_count: Option<usize>,
        #[bpaf(long, short)]
        file: Option<String>,
        #[bpaf(long)]
        stream: Option<String>,  // stdout / stderr / both
    },
}
```

### Dostępne mock outputy (fixtures)

Każdy to plik tekstowy z deterministycznym outputem:

```
tests/fixtures/outputs/

maven_success.txt         → ~200 linii: [INFO] BUILD SUCCESS
maven_failure.txt         → ~500 linii: 5 errorów, 23 warningi, BUILD FAILURE
cargo_success.txt         → ~50 linii: "Compiling... Finished"
cargo_warnings.txt        → ~80 linii: warningi od rustc + success
cargo_errors.txt          → ~120 linii: errory od rustc
gcc_errors.txt            → ~40 linii: file:line:col: error + warningi
generic_long_output.txt   → ~10000 linii losowego tekstu (test truncation)
custom.txt                → user-definable
```

---

## 11. Krytyczna ocena planu

### Zalety

1. **Modułowa architektura** — każdy komponent (exec, parse, storage, config) jest izolowany i testowalny
2. **Konfiguracja w TOML** — łatwo rozszerzać o nowe narzędzia bez zmiany kodu; społeczność może dodawać parserki
3. **mock-command** — kluczowy dla TDD; pozwala testować wszystkie scenariusze bez realnych narzędzi
4. **Trzy tryby działania** (sync/auto/bg) — dają LLM-owi elastyczność
5. **Krótkie task-id** — wygodne do kopiowania/wklejania (8 znaków)
6. **Raw logi zawsze zapisane** — LLM może wrócić do szczegółów, ale domyślnie dostaje tylko esencję
7. **Heurystyczny fallback** — działa nawet dla nieznanych komend (np. custom skrypt bash)

### Ryzyka i wyzwania

| Ryzyko | Poziom | Mitigacja |
|--------|--------|-----------|
| **Background mode** — forkowanie procesów na macOS ma ograniczenia (brak `fork()` w standardzie POSIX na macOS? Jest, ale sandboxing może blokować) | **Wysokie** | Zamiast forka użyj `Command::new` z detachem przez `std::process::Command` i shell `nohup`; lub spawnuj proces zarządzający przez `daemonize` crate |
| **Auto mode timeout** — jak elegancko przejść z trybu online na background? Musimy czytać pipe w async, a po timeoutcie odłączyć proces | **Średnie** | Użyj `tokio::process::Command` + timeout na select; po przekroczeniu timeoutu: persist state, spawn child watcher, zwróć task-id |
| **Wielkość outputu** — niektóre komendy (grep -r, find /) mogą wygenerować GB danych | **Średnie** | Limit outputu: domyślnie 10MB na task (konfigurowalny); po przekroczeniu — truncate + warning w summary |
| **Cross-platform** — ścieżki, sygnały, procesy różnią się między Linux/macOS/Windows | **Średnie** | MVP tylko na macOS/Linux; Windows z opcjonalnym wsparciem przez `winapi` |
| **Wydajność regexp** — parsowanie 100k linii z 20 regexami może być wolne | **Niskie** | Regex zostają skompilowane raz; dla dużych outputów najpierw szybki pre-filter (contains), potem regex |
| **Bezpieczeństwo** — LLM może próbować `smol rm -rf /` lub innych niebezpiecznych komend | **Średnie** | To nie jest problem smol-a; smol deleguje komendę do shella i nie dodaje własnych uprawnień. Odpowiedzialność za bezpieczeństwo leży po stronie frameworka wywołującego LLM |
| **Konflikt flag** — `smol ls -la` vs `smol --mode sync ls -la` | **Niskie** | Standard: `smol [opcje globalne] -- komenda [args]`. Wszystko po `--` to args komendy. BPaf wspiera to natywnie. |

### Nierozstrzygnięte pytania

1. **Czy używać SQLite zamiast registry.toml?** SQLite daje lepszą wydajność przy setkach tasków, atomic writes, query o statusy. TOML jest prostszy na start. **Decyzja**: zacząć od registry.toml, w drodze refactora dodać SQLite.
2. **Async czy sync?** Tokio daje async I/O dla pipe'ów, co ułatwia auto-mode (select między timeoutem a zakończeniem procesu). Ale dodaje zależność. **Decyzja**: użyć Tokio tylko dla exec modułu; reszta może być sync.
3. **Czy parsery mają być w TOML czy w kodzie Rust?** TOML jest extensible, ale ogranicza co można wyrazić (np. kontekstowe regexpy). **Decyzja**: TOML dla 90% przypadków, Rust parsery dla skomplikowanych (np. Maven — trzeba śledzić stan między liniami). To wspiera plugin architecture.
4. **Limit outputu — truncate czy stream?** Jeśli LLM dostaje output w trakcie (stream), może wcześniej przerwać zadanie. **Decyzja**: najpierw collect + summarize dla sync mode; dla bg mode append do pliku.

### Oszacowanie implementacji (w godzinach)

| Faza | Zadania | Czas |
|------|---------|------|
| 1 | Core: CLI, task_id, error types, storage paths | 2-3h |
| 2 | Exec: spawner, watcher, output capture (sync) | 3-4h |
| 3 | Parse: detector, engine, generic parser, regex | 4-5h |
| 4 | Config: loader, parser config TOML, maven.toml | 2-3h |
| 5 | Storage: registry, meta.toml, log files | 2-3h |
| 6 | Mock: mock-command implementation + fixtures | 2-3h |
| 7 | CLI: status/log/list/cancel subcommands | 2-3h |
| 8 | Background: auto mode, bg mode, detach | 4-5h |
| 9 | Parsery: Maven, Cargo, GCC (prawdziwe) | 4-5h |
| 10 | Testy e2e + integracyjne + dokumentacja | 4-6h |
| | **Razem (przybliżony)** | **29-40h** |

### Wnioski

Plan jest **realny i dobrze ustrukturyzowany**. Kluczowe ryzyko to background mode (detach procesu) i auto-mode (async pipe reading). Te dwa obszary wymagają najwięcej uwagi podczas implementacji. Reszta to standardowe zadania w Rust: CLI, regex, TOML, obsługa plików.

Najsilniejszą stroną planu jest **system parserów oparty na TOML** — społeczność może dodawać wsparcie dla nowych narzędzi bez znajomości Rusta, a LLM-y dostają tylko esencję outputu bez szumu.

**Gotów do rozpoczęcia implementacji.** Jeśli chcesz, mogę przygotować pierwszy task — strukturę projektu Cargo + core types + mock-command.
