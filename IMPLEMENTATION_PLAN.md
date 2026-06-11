# Plan implementacji brakujących elementów — `smol`

**Data**: 2026-06-11
**Wersja**: v0.1.0 → v0.2.0 (plan)
**Autor**: Analiza konkurencyjna

---

## Spis treści

1. [Wprowadzenie i metodologia](#1-wprowadzenie-i-metodologia)
2. [Lista brakujących elementów (priorytety)](#2-lista-brakujących-elementów)
3. [Ficzery — szczegółowa analiza + implementacja](#3-ficzery)
   - [P1: MCP Server](#31-mcp-server)
   - [P1: Token counting i redukcja](#32-token-counting-i-redukcja)
   - [P1: Biblioteka parserów (30+)](#33-biblioteka-parserów-30)
   - [P2: Shell auto-completion (bash/zsh/fish)](#34-shell-auto-completion)
   - [P2: Homebrew/Scoop pakiety](#35-homebrew-pakiety)
   - [P2: Hook do AI CLI (tryb agenta)](#36-hook-do-ai-cli-tryb-agenta)
   - [P2: SQLite backend](#37-sqlite-backend)
   - [P2: Testy integracyjne/E2E + 100% coverage](#38-testy-Integracyjne)
   - [P3: Full-text search logów](#39-full-text-search)
   - [P3: Git sync parserów/konfiguracji](#310-git-sync)
   - [P3: Export/Import tasków](#311-exportimport)
   - [P3: Multi-tenancy](#312-multi-tenancy)
   - [P3: Signal handling i daemonizacja](#313-signal-handling)
4. [Zmiany w architekturze](#4-zmiany-w-architekturze)
5. [Zależności w Cargo.toml](#5-zależności)
6. [Krytyczna ocena i review](#6-krytyczna-ocena)

---

## 1. Wprowadzenie i metodologia

### Cel dokumentu

Określenie priorytetów, sposobu implementacji i analizy konkurencyjnej dla każdego brakującego elementu w `smol`.

### Metodologia

Każdy ficzer zawiera:
- **Opis** — co dokładnie trzeba zrobić
- **Analiza konkurencyjna** — jak robią to Shelly, Squeez, Snip CLI, Snip Notes
- **Podejście implementacyjne** — konkretne kroki, pliki, structy
- **Szacowany czas** — w godzinach

### Priorytety

| Priorytet | Opis |
|-----------|------|
| **P1** | Krytyczne — konkurencja ma, bez tego smol wypada gorzej |
| **P2** | Ważne — poprawia UX, dojrzałość, dystrybucję |
| **P3** | Dodatkowe — zwiększa przewagę konkurencyjną |

---

## 2. Lista brakujących elementów

| # | Element | Priorytet | Konkurent(y) z tym ficzerem | Czas (h) |
|---|---------|-----------|-----------------------------|----------|
| 1 | MCP Server | **P1** | Shelly, Squeez | 8-12 |
| 2 | Token counting + redukcja | **P1** | Squeez (95% redukcji) | 6-10 |
| 3 | Biblioteka parserów (30+) | **P1** | Squeez (30+), Shelly (kilka) | 10-15 |
| 4 | Shell auto-completion | **P2** | Snip (mehran-prs) | 4-6 |
| 5 | Homebrew/Scoop pakiety | **P2** | Snip Notes (matheuzgomes) | 2-4 |
| 6 | Hook do AI CLI | **P2** | Squeez (5 hostów), Rtk | 6-10 |
| 7 | SQLite backend | **P2** | Snip Notes (matheuzgomes) | 6-8 |
| 8 | Testy integracyjne/E2E | **P2** | Squeez (356 testów) | 8-12 |
| 9 | Full-text search logów | **P3** | Snip Notes (FTS4) | 4-6 |
| 10 | Git sync parserów/config | **P3** | Snip (mehran-prs) | 3-5 |
| 11 | Export/Import tasków | **P3** | Snip Notes (JSON/MD) | 3-4 |
| 12 | Multi-tenancy (soft-link) | **P3** | Snip (mehran-prs) | 2-3 |
| 13 | Signal handling + daemonizacja | **P3** | — (nikt nie ma bg tasków) | 4-6 |
| | **Razem** | | | **66-101h** |

---

## 3. Ficzery — szczegółowa analiza + implementacja

---

### 3.1 MCP Server

**Priorytet**: P1 | **Czas**: 8-12h | **Zależności**: brak

#### Opis

Model Context Protocol (MCP) server pozwala AI agentom (Claude Code, OpenCode, itp.) komunikować się z `smol` przez ustandaryzowany protokół JSON-RPC 2.0. Dzięki temu agent może:
- Uruchamiać komendy przez `smol` z poziomu MCP
- Sprawdzać status tasków
- Pobierać logi
- Czyścić stare taski

#### Analiza konkurencyjna

**Shelly**:
- MCP server w `shelly-mcp/` — osobny binary
- Implementacja od zera (nie używa `fastmcp` czy `mcp-server` crate'a)
- Eksponuje `execute_command` jako narzędzie MCP
- Uruchamiany przez `shelly-dev.sh` lub `cargo run --bin shelly-mcp`
- Przekazuje `command`, `working_dir`, `exact`, `settings`, `env` w requeście

**Squeez**:
- 13 read-only MCP toolów: `squeez_recent_calls`, `squeez_seen_files`, `squeez_seen_errors`, `squeez_session_summary`, `squeez_session_stats`, `squeez_agent_costs`, `squeez_session_efficiency`, `squeez_prior_summaries`, `squeez_search_history`, `squeez_file_history`, `squeez_session_detail`, `squeez_protocol`
- Hand-rolled JSON-RPC 2.0, bez zależności (`libc`-only)
- Uruchamiany przez `squeez mcp`
- Wszystkie narzędzia read-only (brak side effects)

#### Podejście implementacyjne

1. **Nowy binarny target**: `smol-mcp` w `Cargo.toml`
2. **Protokół**: JSON-RPC 2.0 przez stdio (najprostszy, zgodny ze standardem MCP)
3. **Narzędzia (tools)**:
   - `smol_run` — uruchom komendę (sync/auto/bg)
   - `smol_status` — status taska
   - `smol_log` — pobierz logi
   - `smol_list` — lista tasków
   - `smol_cancel` — anuluj task
   - `smol_clean` — wyczyść stare
4. **Implementacja**: albo hand-rolled (jak Squeez), albo użyć `mcp-server` crate'a (jeśli stabilny)

```rust
// src/mcp/mod.rs
pub struct SmolMcpServer {
    // stan serwera
}

impl SmolMcpServer {
    pub async fn run() -> Result<()> {
        // JSON-RPC 2.0 loop na stdio
    }

    async fn handle_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "smol_run" => self.cmd_run(request.params),
            "smol_status" => self.cmd_status(request.params),
            // ...
        }
    }
}
```

5. **Dokumentacja**: instrukcja dodania do Claude Code / OpenCode przez `claude mcp add`

---

### 3.2 Token counting i redukcja

**Priorytet**: P1 | **Czas**: 6-10h | **Zależności**: nowy crate (opcjonalnie `tiktoken-rs`)

#### Opis

Obliczanie liczby tokenów w outputcie przed i po przetworzeniu przez `smol`. To kluczowy KPI — Squeez reklamuje się "95% redukcji", bez tego ciężko porównywać.

#### Analiza konkurencyjna

**Squeez**:
- 4-stage pipeline: smart_filter → dedup → grouping → truncation
- Token counting przez `chars / 4` (zgodne z ~4 chars/token dla Claude)
- Benchmark: 28 scenariuszy × 5 iteracji
- Wynik: 90.7% redukcji (164,224 tk → 15,206 tk)
- Dodatkowo: adaptive intensity (Full/Ultra w zależności od budżetu)
- Cross-call dedup: FNV-1a hash + MinHash fuzzy (Jaccard ≥ 0.85)

**Snip Notes** (matheuzgomes): Brak — nie dotyczy (tool do notatek)

**Shelly**: Brak jawnego token count, tylko summarization

#### Podejście implementacyjne

1. **Token counter**: funkcja `count_tokens(text: &str) -> usize`
   - Prosta: `text.len() / 4` (zgodne z Claude tokenizer approximation)
   - Opcjonalnie: `tiktoken-rs` crate dla dokładniejszego liczenia (cl100k_base)

2. **Dodanie do Summary**:
   ```rust
   pub struct Summary {
       // ... existing fields
       pub input_tokens: usize,
       pub output_tokens: usize,
       pub compression_ratio: f64,
   }
   ```

3. **Raport w summary**: `[tokens: 2843 → 47 (-98%)]`

4. **Benchmark**: dodać benchmark scenariuszy (wzorem Squeez):
   - `maven_success`, `maven_failure`, `cargo_build`, `gcc_errors`
   - `git_log_large`, `docker_build`, `npm_install`
   - Pomiar latency, reduction, quality (signal terms preserved)

5. **Optional: dedup pipeline**:
   - exact-hash dedup (FNV-1a) — tani, szybki
   - fuzzy dedup (MinHash + Jaccard) — bardziej złożony, P2

---

### 3.3 Biblioteka parserów (30+)

**Priorytet**: P1 | **Czas**: 10-15h | **Zależności**: obecny system TOML

#### Opis

Obecnie `smol` ma 6 parserów (maven, cargo, gcc, docker, git, generic). Squeez ma 30+ handlerów. Potrzebujemy znacząco rozszerzyć bibliotekę.

#### Analiza konkurencyjna

**Squeez** — pełna lista handlerów:
- Git: `git`
- Docker: `docker`, `docker-compose`, `podman`
- Package managers: `npm`, `pnpm`, `bun`, `yarn`
- Build systems: `make`, `cmake`, `gradle`, `mvn`, `xcodebuild`, `cargo`, `next`
- Test runners: `cargo test`, `jest`, `vitest`, `pytest`, `nextest`, `playwright`, `bun test`
- TypeScript/linters: `tsc`, `eslint`, `biome`
- Cloud CLIs: `kubectl`, `gh`, `aws`, `gcloud`, `az`, `wrangler`
- Databases: `psql`, `prisma`, `mysql`, `drizzle-kit`
- Filesystem: `find`, `ls`, `du`, `ps`, `env`, `lsof`, `netstat`
- JSON/YAML/IaC: `jq`, `yq`, `terraform`, `tofu`, `helm`, `pulumi`
- Text: `grep`, `rg`, `awk`, `sed`
- Network: `curl`, `wget`
- Runtimes: `node`, `python`, `ruby`

**Shelly**: cargo handler, TypeScript-based. Dodaje `--quiet`, filtruje warningi.

#### Podejście implementacyjne

1. **Nowe parsery TOML** — priorytetyzacja:

   **Runda 1 (P0, natychmiast)**:
   - `gradle` / `gradlew` — build tool (częsty w AI workflows)
   - `npm` / `npx` — node package manager
   - `yarn` / `pnpm` — alternatywy npm
   - `go` — `go build`, `go test`
   - `tsc` — TypeScript compiler
   - `rustc` — Rust compiler (osobno od cargo)
   - `python` — Python tracebacki, pip, pytest

   **Runda 2 (P1, tydzień 2)**:
   - `make` — GNU Make (obecnie ma TOML?)
   - `cmake` — CMake
   - `kubectl` — Kubernetes
   - `terraform` — IaC
   - `ansible` — config management
   - `curl` — network
   - `node` — Node.js runtime
   - `jest` / `pytest` / `vitest` — test runners
   - `eslint` / `biome` — lintery
   - `find` / `rg` / `grep` — search tools

   **Runda 3 (P2, później)**:
   - `psql`, `mysql` — databases
   - `gh` — GitHub CLI
   - `aws`, `gcloud`, `az` — cloud CLIs
   - `docker-compose` / `podman`
   - `helm`, `pulumi`, `tofu`
   - `next build`, `vite`

2. **Strategia**: Każdy parser to plik TOML w `parsers/` z:
   - `detection.command_prefix` lub `detection.heuristic`
   - `ignore_patterns` specyficzne dla narzędzia
   - `error_patterns`, `warning_patterns`
   - `status_patterns`

3. **Testy**: Dla każdego parsera — mock output + test jednostkowy
   - Pliki `tests/fixtures/outputs/<tool>_*.txt`
   - Test integracyjny: `smol mock-command --name <tool>_*`

4. **Generator parsera** (opcjonalnie, P3):
   - Narzędzie `smol generate-parser` które analizuje output i proponuje konfigurację TOML

---

### 3.4 Shell auto-completion

**Priorytet**: P2 | **Czas**: 4-6h | **Zależności**: opcjonalnie `clap` zamiast hand-rolled parsera

#### Opis

Auto-completion dla `smol` w bash/zsh/fish — użytkownik wpisuje `smol st` + Tab → `smol status`.

#### Analiza konkurencyjna

**Snip (mehran-prs)** — wzorcowa implementacja:
- `snip completion bash` → generuje skrypt completion
- `snip completion zsh` → jw.
- `snip completion fish` → jw.
- Dodatkowo: **fuzzy completion przez fzf** — `snip **` + Tab → fzf z listą snippetów
- Implementacja: wbudowana w Go (`cobra.Command.GenCompletion()`)

#### Podejście implementacyjne

1. **Opcja A (szybka)**: Dodać podkomendę `smol completion <shell>`:
   - Generuje statyczne completion (subkomendy: `status`, `log`, `list`, `cancel`, `clean`, `help`)
   - Dla `status`/`log`/`cancel` — completion task IDs (dynamiczne)
   - Implementacja ręczna (shell script jako string w Ruście)

2. **Opcja B (lepsza)**: Użyć `clap` z feature `derive` i `completion`:
   - Zamienić hand-rolled parser na `clap`
   - Automatyczny completion przez `clap_completion` crate
   - Większy koszt zmiany, ale lepsze utrzymanie

3. **Fuzzy completion (P3)**: Integracja z fzf dla task-id i parserów

---

### 3.5 Homebrew/Scoop pakiety

**Priorytet**: P2 | **Czas**: 2-4h | **Zależności**: GitHub Actions

#### Opis

Dystrybucja `smol` przez popularne package mangery: Homebrew (macOS), Scoop (Windows), cargo (już jest).

#### Analiza konkurencyjna

**Snip Notes** (matheuzgomes): Homebrew (`brew install snip`), Scoop (`scoop install snip`)
- Formula w `Homebrew/homebrew-core`
- Manifest w `ScoopInstaller/Main`

**Squeez**: npm (`npm install -g squeez`), cargo (`cargo install squeez`), curl pipe (`install.sh`)

#### Podejście implementacyjne

1. **Homebrew**:
   - Stworzyć formula w `Homebrew/homebrew-core` lub własnym tapie `nnar1o/homebrew-tap`
   - Formula Rust: używa `cargo install` lub pobiera pre-built binary
   - Własny tap: `brew tap nnar1o/tap && brew install smol`

2. **Scoop** (Windows):
   - Manifest w `ScoopInstaller/Main` lub własnym buckecie
   - Pobiera pre-built binary z GitHub Releases

3. **Dodatkowo**: npm (`npx smol`) — jeśli ktoś woli Node.js ecosystem
   - Wrapper skrypt npm, który `cargo install` lub pobiera binary

---

### 3.6 Hook do AI CLI (tryb agenta)

**Priorytet**: P2 | **Czas**: 6-10h | **Zależności**: zrozumienie hook API Claude Code / OpenCode

#### Opis

Hookowanie `smol` bezpośrednio do AI CLI (Claude Code, OpenCode) tak, aby każde wywołanie komendy było automatycznie wrapowane przez `smol`.

#### Analiza konkurencyjna

**Squeez**:
- Hook dla 5 hostów: Claude Code, Copilot CLI, OpenCode, Gemini CLI, Codex CLI
- `PreToolUse` hook: `git status` → `squeez wrap git status`
- `SessionStart`: finalizuje poprzednią sesję, wstrzykuje personę
- `PostToolUse`: śledzi wyniki, przepisuje output Read/Grep
- `SubagentStop`: dedup cross-call
- Rejestracja przez `squeez setup` (auto-detekcja hostów)

**Rtk**: Hook proxy — podmienia komendy przed wykonaniem

#### Podejście implementacyjne

1. **OpenCode plugin**:
   - Skrypt JS w `~/.config/opencode/plugins/smol.js`
   - Przed każdym `Bash` — wrapuj komendę przez `smol <command>`
   - Po każdym `Bash` — parsuj output przez smol

2. **Claude Code hook**:
   - Skrypt `PreToolUse` w `~/.claude/hooks/`
   - Przekierowuje `Bash` przez `smol`

3. **Instalacja**:
   ```sh
   smol setup --host=opencode   # instaluje plugin
   smol setup --host=claude     # instaluje hook
   smol uninstall               # usuwa wszystkie
   ```

---

### 3.7 SQLite backend

**Priorytet**: P2 | **Czas**: 6-8h | **Zależności**: `rusqlite` lub `sqlx`

#### Opis

Zamiana registry.toml na SQLite dla lepszej wydajności, atomowych zapisów i możliwości zaawansowanych zapytań.

#### Analiza konkurencyjna

**Snip Notes** (matheuzgomes):
- SQLite z FTS4 (full-text search)
- `90-127ns` na operację
- 100% test coverage

**smol obecnie**: TOML-based registry (`registry.toml`), każdy task to osobny katalog

#### Podejście implementacyjne

1. **Nowa baza**: `~/.smol/smol.db` — SQLite
2. **Tabele**:
   ```sql
   CREATE TABLE tasks (
       id TEXT PRIMARY KEY,
       command TEXT NOT NULL,
       mode TEXT NOT NULL,
       status TEXT NOT NULL DEFAULT 'running',
       created_at TEXT NOT NULL,
       completed_at TEXT,
       exit_code INTEGER,
       duration_sec INTEGER,
       error_count INTEGER DEFAULT 0,
       warning_count INTEGER DEFAULT 0,
       pid INTEGER,
       background_pid INTEGER
   );

   CREATE TABLE errors (
       id INTEGER PRIMARY KEY AUTOINCREMENT,
       task_id TEXT NOT NULL REFERENCES tasks(id),
       line_number INTEGER NOT NULL,
       file TEXT,
       file_line INTEGER,
       column INTEGER,
       content TEXT NOT NULL
   );

   CREATE TABLE warnings (
       id INTEGER PRIMARY KEY AUTOINCREMENT,
       task_id TEXT NOT NULL REFERENCES tasks(id),
       line_number INTEGER NOT NULL,
       file TEXT,
       file_line INTEGER,
       content TEXT NOT NULL
   );

   CREATE VIRTUAL TABLE tasks_fts USING fts4(content, command);
   ```

3. **Migracja**: Skrypt `smol migrate` który przenosi dane z registry.toml do SQLite
4. **Backward compatibility**: Przez 1-2 wersje wspierać oba backendy (wykrywanie który istnieje)

---

### 3.8 Testy integracyjne/E2E + 100% coverage

**Priorytet**: P2 | **Czas**: 8-12h | **Zależności**: istniejący system mock-command

#### Opis

Obecnie: 54 testy jednostkowe, 0 testów integracyjnych, 0 testów E2E, katalog `tests/` pusty.
Potrzebujemy pełnego pokrycia: unit → integration → e2e.

#### Analiza konkurencyjna

**Squeez**: 356 testów, 37 suites, CI na każdym push/PR
**Shelly**: test framework w TOML dla handlerów (`.shelly/tests/<handler>/*.toml`)
**Snip (mehran-prs)**: Go test + coverage, codecov.io (97% coverage)

#### Podejście implementacyjne

1. **Struktura testów** (zgodnie z PLAN.md):
   ```
   tests/
   ├── e2e/
   │   ├── cli_test.rs              ← testy CLI (smol <cmd>)
   │   ├── modes_test.rs            ← sync/auto/bg
   │   └── task_commands_test.rs    ← status/log/list/cancel/clean
   ├── integration/
   │   ├── parser_detection_test.rs ← detekcja narzędzia
   │   ├── maven_parser_test.rs     ← mock output → parsing
   │   ├── cargo_parser_test.rs
   │   ├── gcc_parser_test.rs
   │   ├── npm_parser_test.rs
   │   ├── generic_parser_test.rs
   │   └── output_capture_test.rs   ← stdout/stderr capture, truncation
   ├── unit/
   │   ├── task_id_test.rs
   │   ├── config_test.rs
   │   ├── storage_test.rs
   │   ├── summarizer_test.rs
   │   └── heuristic_test.rs
   └── fixtures/
       ├── outputs/                 ← mock outputi (10-15 plików)
       └── configs/                 ← testowe parsery TOML
   ```

2. **Testy E2E przez mock-command**:
   ```rust
   #[test]
   fn test_sync_maven_success() {
       let mut cmd = Command::cargo_bin("smol").unwrap();
       cmd.arg("mock-command").arg("--name").arg("maven_success");
       cmd.assert().stdout(predicates::str::contains("success"));
   }
   ```

3. **Coverage**: `cargo tarpaulin` lub `cargo-llvm-cov` do pomiaru
4. **CI**: GitHub Actions — testy na macOS + Linux + Windows

---

### 3.9 Full-text search logów

**Priorytet**: P3 | **Czas**: 4-6h | **Zależności**: SQLite backend (FTS4) lub grep

#### Opis

Wyszukiwanie pełnotekstowe we wszystkich logach tasków — znajdź task gdzie pojawił się błąd "NullPointerException".

#### Analiza konkurencyjna

**Snip Notes** (matheuzgomes):
- SQLite FTS4 (`CREATE VIRTUAL TABLE ... USING fts4`)
- `snip find "NullPointer"` → wyniki w ~90-127ns

#### Podejście implementacyjne

1. **Z SQLite backendem**: FTS4 na tabeli `tasks` (command) i osobna tabela dla log contentu
2. **Bez SQLite**: `grep -r` po `~/.smol/tasks/*/output.log`
3. **Komenda**: `smol search <query>` lub `smol find <query>`
4. **Output**:
   ```
   task-id:a3f9k2   mvn clean install   2026-06-11   error "NullPointer" at line 142
   task-id:b8xR1a   cargo build         2026-06-10   warning "unused" at line 55
   ```

---

### 3.10 Git sync parserów/konfiguracji

**Priorytet**: P3 | **Czas**: 3-5h | **Zależności**: Git CLI

#### Opis

Synchronizacja parserów i konfiguracji `smol` przez Git — społeczność może share'ować własne parsery.

#### Analiza konkurencyjna

**Snip (mehran-prs)**:
- `snip sync` — `git pull`, `git add -A`, `git commit -m "..."`, `git push`
- Działa na `SNIP_DIR` (katalog snippetów)
- Wymaga ręcznej inicjalizacji repo

#### Podejście implementacyjne

1. **Komenda**: `smol parsers sync` — sync parsers z remote repo
2. **Domyślny remote**: `github.com/nnar1o/smol-parsers`
3. **Przepływ**:
   ```sh
   smol parsers init           # git init w ~/.smol/parsers/
   smol parsers remote add ... # ustaw remote
   smol parsers sync           # pull + push
   smol parsers publish        # opublikuj własny parser
   ```

4. **Parsery community**: Repozytorium `smol-parsers` gdzie każdy może wrzucić PR z parserem TOML

---

### 3.11 Export/Import tasków

**Priorytet**: P3 | **Czas**: 3-4h | **Zależności**: Storage backend

#### Opis

Eksport tasków do JSON/Markdown i import z powrotem. Przydatne do backupu i analizy offline.

#### Analiza konkurencyjna

**Snip Notes** (matheuzgomes):
- `snip export --json` → JSON
- `snip export --markdown` → Markdown
- `snip import <file>` → import z pliku

#### Podejście implementacyjne

1. **Export taska**:
   ```sh
   smol export <task-id> --json     # JSON z meta + logi
   smol export <task-id> --markdown # czytelny raport
   smol export --all                # wszystkie taski
   ```

2. **Import**:
   ```sh
   smol import <file.json>     # import taska z JSON
   ```

3. **Format JSON**:
   ```json
   {
     "id": "a3f9k2",
     "command": "mvn clean install",
     "status": "failed",
     "errors": 5,
     "warnings": 23,
     "stdout": "[INFO] ...",
     "stderr": "[ERROR] ..."
   }
   ```

---

### 3.12 Multi-tenancy (soft-link)

**Priorytet**: P3 | **Czas**: 2-3h | **Zależności**: brak

#### Opis

Możliwość stworzenia wielu "instancji" `smol` przez soft-link: `smol-build`, `smol-test`, każda z własną konfiguracją i katalogiem tasków.

#### Analiza konkurencyjna

**Snip (mehran-prs)**:
- `ln -s $(which snip) /usr/local/bin/tasks`
- `tasks` czyta `TASKS_DIR` zamiast `SNIP_DIR`
- Auto-kompletion działa dla każdej instancji

#### Podejście implementacyjne

1. **Wykrywanie nazwy binary**: `smol` sprawdza `argv[0]` (nazwa wywołania)
2. **Konwersja na wielkie litery**: `smol-build` → `SMOL_BUILD_*` env vars
3. **Np**: `smol-build` czyta `SMOL_BUILD_TASKS_DIR` zamiast `SMOL_TASKS_DIR`
4. **Przypadek użycia**: `smol-build` zawsze w trybie sync dla buildów, `smol-test` z innym timeoutem

---

### 3.13 Signal handling + daemonizacja

**Priorytet**: P3 | **Czas**: 4-6h | **Zależności**: `tokio` lub `signal-hook`

#### Opis

Obecnie `smol` nie obsługuje sygnałów (SIGTERM, SIGINT) i nie daemonizuje procesów w tle prawidłowo.

#### Analiza konkurencyjna

Żaden konkurent nie ma background tasków, więc to unikalna cecha `smol`. Ale implementacja jest niedoskonała:
- `spawn_background_task` dropuje `Child` handle (proces może dostać SIGPIPE)
- Brak `setsid` / `fork` — proces nie jest prawdziwie odłączony
- Brak PID file

#### Podejście implementacyjne

1. **Signal handling** (główny proces):
   - `SIGINT` / `SIGTERM` → anuluj bieżący task + exit
   - Użyc `signal-hook` lub `tokio::signal`

2. **Daemonizacja** (background mode):
   - Użyć `daemonize` crate lub `std::process::Command` z `nohup`
   - Zapisać PID do `meta.toml` (już jest)
   - Proces potomny: `setsid`, zamknięcie stdio rodzicielskiego

3. **Czyszczenie przy starcie**:
   - Sprawdź czy PID z `meta.toml` żyje
   - Jeśli nie → oznacz task jako zakończony (lazy update)

---

## 4. Zmiany w architekturze

### Obecna struktura

```
src/
├── main.rs
├── lib.rs
├── cli/          ← args, commands (hand-rolled parser)
├── core/         ← typy wspólne
├── config/       ← ładowanie configu TOML
├── exec/         ← spawner, watcher, backgrounder
├── mock/         ← mock-command
├── parse/        ← detector, engine, generic, summarizer
└── storage/      ← paths, registry, task_store
```

### Nowa struktura (docelowa)

```
src/
├── main.rs
├── lib.rs
├── cli/              ← to samo, ale z opcjonalnym clap
├── core/             ← + token count, + fts
├── config/           ← + git sync
├── exec/             ← + signal handling, + daemonize
├── mock/             ← to samo
├── parse/            ← + 24 nowe parsery TOML
├── storage/          ← + SQLite backend
├── mcp/              ← NOWY: MCP server
├── hook/             ← NOWY: setup/install hooks
├── export/           ← NOWY: export/import
└── completions/      ← NOWY: shell completion

parsers/              ← + 24 nowe pliki .toml
  ├── maven.toml
  ├── cargo.toml
  ├── ...
  ├── gradle.toml     ← NOWY
  ├── npm.toml        ← NOWY
  ├── go.toml         ← NOWY
  ├── python.toml     ← NOWY
  ├── kubectl.toml    ← NOWY
  ├── terraform.toml  ← NOWY
  └── ...

smol-mcp/             ← NOWY: osobny binary MCP
  └── src/main.rs

tests/                ← wypełnić
  ├── e2e/
  ├── integration/
  ├── unit/
  └── fixtures/
```

---

## 5. Zależności w Cargo.toml

### Obecne

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
regex = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
rand = "0.8"
```

### Docelowe (z nowymi ficzerami)

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
regex = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
rand = "0.8"

# P1: MCP Server
# Opcja hand-rolled: brak zależności (JSON-RPC przez stdio)
# Opcja z crate: mcp-server = "0.1"   # jeśli stabilny

# P1: Token counting
# Opcja basic: brak (chars/4)
# Opcja precise: tiktoken-rs = "0.5"  # cl100k_base

# P2: SQLite
# rusqlite = { version = "0.31", features = ["bundled"] }

# P2: Shell completion (jeśli clap)
# clap = { version = "4", features = ["derive", "env"] }
# clap_complete = "4"

# P3: Signal handling
# signal-hook = "0.3"

# P3: Daemonize (opcjonalnie)
# daemonize = "0.5"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
# P2: Coverage
# tarpaulin = "0.27"        # lub llvm-cov
```

### Uzasadnienie wyboru zależności

| Zależność | Uzasadnienie | Ryzyko |
|-----------|--------------|--------|
| `rusqlite` | Dojrzały, bezpieczny typowo, wspiera FTS4 | Zwiększa rozmiar binary (bundled SQLite ~2MB) |
| `clap` | Automatyczny completion, lepsze błędy, `--help` | Zmiana architektury CLI (breaking change) |
| `tiktoken-rs` | Dokładne liczenie tokenów (BPE) | Dodatkowa zależność, można policzyć `chars/4` |
| `signal-hook` | Bezpieczna obsługa sygnałów | Minimalna zależność |
| `mcp-server` | Gotowy protokół MCP | Może być niestabilny (early stage) |

---

## 6. Krytyczna ocena

### 6.1 Mocne strony planu

1. **Dobrze zbadana konkurencja** — każdy ficzer ma analizę jak robią to Shelly, Squeez, Snipy. Można kopiować sprawdzone rozwiązania.

2. **Priorytetyzacja oparta na rynku** — P1 to rzeczy które konkurencja ma i które są kluczowe dla pozycjonowania. P2-P3 to przewagi które mogą być unikalne.

3. **System parserów TOML pozostaje przewagą** — konkurencja (Shelly) wymaga TypeScript, Squeez ma wbudowane handlery. TOML jest prostszy dla community.

4. **Unikalna cecha** (task management z bg/auto/sync + task-id) pozostaje nieruszona i wzmacniana przez SQLite i export.

5. **MCP Server + Hook** pozwolą smol być używanym nie tylko jako bezpośrednie CLI, ale jako narzędzie dla AI agentów przez standardowy protokół.

### 6.2 Ryzyka i słabe strony

| Ryzyko | Poziom | Mitigacja |
|--------|--------|-----------|
| **Zbyt ambitny zakres** (66-101h) | 🔴 WYSOKIE | Podział na fazy: Faza 1 = P1 tylko (24-37h), Faza 2 = P2 (20-28h), Faza 3 = P3 |
| **Zmiana CLI na clap** może być breaking change | 🟡 ŚREDNIE | Zachować backward compatibility, dodać clap jako opcję |
| **SQLite zwiększa rozmiar binary** | 🟢 NISKIE | `bundled` feature = ~2MB więcej. Do zaakceptowania |
| **MCP Server — brak stabilnego crate'a** | 🟡 ŚREDNIE | Hand-rolled JSON-RPC 2.0 (jak Squeez). Proste i bez zależności |
| **Hook do AI CLI** — API może się zmienić | 🟡 ŚREDNIE | Śledzić zmiany w Claude Code/OpenCode hook API |
| **Squeez ma 135★ i 356 testów** — duża przewaga | 🟡 ŚREDNIE | Skupić się na unikalnych cechach (task management, TOML parsers) |
| **Testy E2E przez mock-command** — działają tylko w debug | 🟢 NISKIE | Dodać `--enable-mock` flagę do release build |

### 6.3 Rekomendowana kolejność implementacji

#### Faza 1: Krytyczne (P1) — 24-37h

| Krok | Co | Czas | Zależności |
|------|----|------|------------|
| 1.1 | **MCP Server** (hand-rolled JSON-RPC) | 8-12h | — |
| 1.2 | **Token counting** (basic) | 3-4h | — |
| 1.3 | **Nowe parsery: Runda 1** (gradle, npm, go, tsc, python) | 5-7h | — |
| 1.4 | **Integracja MCP + smol-core** | 4-6h | 1.1, 1.2 |
| 1.5 | **Benchmark token reduction** | 4-6h | 1.2, 1.3 |
| 1.6 | **Nowe parsery: Runda 2** (make, cmake, kubectl, terraform, jest, eslint) | 5-8h | 1.3 |

#### Faza 2: Ważne (P2) — 20-28h

| Krok | Co | Czas | Zależności |
|------|----|------|------------|
| 2.1 | **SQLite backend** | 6-8h | — |
| 2.2 | **Testy integracyjne + E2E** | 8-12h | 1.3 (parsery), 2.1 |
| 2.3 | **Shell auto-completion** (przez clap) | 4-6h | — |
| 2.4 | **Homebrew/Scoop pakiety** | 2-4h | CI działa |

#### Faza 3: Dodatkowe (P3) — 16-24h

| Krok | Co | Czas | Zależności |
|------|----|------|------------|
| 3.1 | **Hook do AI CLI** (setup/install) | 6-10h | 1.1 (MCP) |
| 3.2 | **Signal handling + daemonizacja** | 4-6h | — |
| 3.3 | **Full-text search** | 4-6h | 2.1 (SQLite) |
| 3.4 | **Git sync, Export/Import, Multi-tenancy** | 8-12h | 2.1 |

### 6.4 Mierniki sukcesu (KPIs)

Po implementacji planu, `smol` powinien:

| Miernik | Obecnie | Cel po implementacji |
|---------|---------|---------------------|
| Liczba parserów | 6 | 30+ |
| Token reduction | nie mierzone | >85% (benchmark) |
| Test coverage | ~30% | >90% |
| Liczba testów | 54 | 200+ |
| MCP integration | ❌ Brak | ✅ Narzędzia MCP |
| Shell completion | ❌ Brak | ✅ bash/zsh/fish |
| Package managers | cargo tylko | cargo + homebrew + scoop |
| AI CLI hooks | ❌ Brak | ✅ OpenCode + Claude Code |
| Obsługa sygnałów | ❌ Brak | ✅ SIGTERM/SIGINT |

### 6.5 Podsumowanie

Plan jest **ambitny ale realistyczny**. Kluczowe decyzje:

1. **MCP Server first** — to najważniejszy brak vs konkurencja (Shelly, Squeez)
2. **TOML > TypeScript** — nie konkurujemy z Shelly na polu handlerów; TOML jest prostszy dla społeczności
3. **Task management jako diferenciator** — nikt nie ma bg/auto/sync + task-id + raw logs. SQLite i FTS wzmocnią tę przewagę
4. **Unikać zależności** — Squeez ma zero runtime deps (tylko `libc`). Dążyć do minimum zależności

**Największe ryzyko**: zakres (66-101h). Sugeruję skupić się na Faza 1 (P1, ~30h) jako v0.2.0, a resztę jako v0.3.0+.

**Największa szansa**: MCP Server + Hook do AI CLI + Task Management = unikalne połączenie, którego nie ma żaden konkurent. Squeez ma hooki i MCP, ale nie ma task managementu. Shelly ma MCP, ale nie ma TOML parserów ani tasków. **smol może być pierwszym toolem który ma wszystko.**

---

## 7. Raport implementacyjny — stan na 2026-06-11

### 7.1 Podsumowanie wykonania

| Faza | Zakres | Status | Testy | Uwagi |
|------|--------|--------|-------|-------|
| **Faza 1** (P1) | MCP Server, Token counting, 14 parserów TOML | ✅ **Zrobione** | 263 łącznie | Hand-rolled JSON-RPC, benchmark 5 scenariuszy, 20 parserów total |
| **Faza 2** (P2) | SQLite, Testy, Shell completion, Homebrew | ✅ **Zrobione** | — | rusqlite bundled, 140+ nowych testów, bash/zsh/fish, formula + GH workflow |
| **Faza 3** (P3) | Hooki, Signal, FTS, Export/Import, Git sync, Multi-tenancy | ✅ **Zrobione** | — | OpenCode JS + Claude Code sh, setsid daemon, smol search, JSON/MD export |
| **+ Bonus** | Acceptance/test result parsing | ✅ **Zrobione** | — | Maven Surefire/Failsafe, Cargo test, Jest — TestResult w Summary, TestPattern w ParserConfig |

### 7.2 Rzeczywiste KPIs vs plan

| Miernik | Plan (cel) | Rzeczywisty | Uwagi |
|---------|------------|-------------|-------|
| Liczba parserów | 30+ | **20** | 6 oryginalnych + 14 nowych. Zamiast 10 kolejnych: parsowanie wyników testów |
| Token reduction | >85% | **liczone** | `estimate_tokens()` przez chars/4, benchmark w `src/bench.rs` |
| Test coverage | >90% | ~**85%** (szac.) | Pokryte: parsery, storage, CLI, mock, task lifecycle, search, export |
| Liczba testów | 200+ | **263** | Unit (unit/), integration (integration/), E2E (e2e/) + doc-tests |
| MCP integration | ✅ | ✅ | 6 narzędzi: run, status, log, list, cancel, clean |
| Shell completion | ✅ bash/zsh/fish | ✅ | `smol completion bash\|zsh\|fish`, statyczne + dynamiczne task-id |
| Package managers | cargo + homebrew + scoop | **cargo + homebrew** | Formula + update script + GH workflow; Scoop — do dodania |
| AI CLI hooks | ✅ OpenCode + Claude Code | ✅ | `smol setup` (auto-detekcja), `smol uninstall` |
| Obsługa sygnałów | ✅ SIGTERM/SIGINT | ✅ | `AtomicBool`, setsid, background survival przez SIGINT |

### 7.3 Rzeczywiste godziny (szacowane)

| Komponent | Godziny | Główne pliki |
|-----------|---------|--------------|
| MCP Server | ~6h | `src/mcp/`, `src/bin/smol-mcp.rs` |
| Token counting + benchmark | ~3h | `src/core/summary.rs`, `src/bench.rs` |
| 14 parserów TOML | ~4h | `parsers/*.toml` (20 plików) |
| SQLite backend | ~5h | `src/storage/sqlite.rs` (730 linii) |
| Shell completion | ~2h | `src/completions/` |
| Homebrew + release workflow | ~2h | `homebrew/smol.rb`, `.github/workflows/` |
| Hook do AI CLI | ~4h | `src/hook/` (OpenCode JS + Claude Code sh) |
| Signal + daemonizacja | ~3h | `src/exec/signal.rs`, `src/exec/backgrounder.rs` |
| Full-text search | ~2h | `src/storage/search.rs` |
| Export/Import | ~2h | `src/storage/export.rs` |
| Git sync + Multi-tenancy | ~2h | `src/config/sync.rs`, `src/storage/paths.rs` |
| Acceptance/test result parsing | ~3h | `src/core/summary.rs`, `src/parse/engine.rs`, parsery Maven/Cargo/Jest |
| Testy (140+ nowych) | ~6h | `tests/` (16 plików) |
| **Razem** | **~44h** | Plan: 66-101h. Oszczędność: ręczny MCP zamiast crate'a, brak clap, brak scoop |

### 7.4 Co jeszcze zostało (opcjonalnie)

| # | Element | Priorytet | Czas |
|---|---------|-----------|------|
| 1 | **Scoop** pakiety (Windows) | Niski | 2h |
| 2 | **10+ dodatkowych parserów** (cmake, pytest, ansible, gh, aws, find, psql, docker-compose, helm, biome) | Średni | 4-6h |
| 3 | **clap migration** — zastąpić hand-rolled parser, lepszy --help, błędy, autocomplete | Niski (breaking) | 4-6h |
| 4 | **Fuzzy completion przez fzf** (`smol **` + Tab) | Niski | 2-3h |
| 5 | **Multi-tenancy demo** — `smol-smol` symlink, fixture acceptance test `mvn verify` | ✅ **Zrobione** | — |
| 5 | **tiktoken-rs** — dokładniejsze liczenie tokenów (BPE cl100k_base) | Niski | 2h |
| 6 | **CI na Windows** w GitHub Actions | Niski | 2h |
| 7 | **code coverage** (cargo-tarpaulin lub cargo-llvm-cov) + badge | Niski | 2h |

### 7.5 Wnioski

- **Fazy 1-3 zrealizowane w ~44h** (vs planowane 66-101h). Oszczędność wynikła z: hand-rolled MCP zamiast crate'a, braku migracji na clap, pominięcia Scoop, skupienia na TOML (szybciej niż TypeScript/rust handlers).
- **Bonus**: parsowanie wyników testów (acceptance tests) — feature który został dodany ad-hoc na życzenie.
- **263 testy, 0 błędów** — jakość potwierdzona.
- **20 parserów** — mniej niż cel 30+, ale zamiast kolejnych parserów dodano parsowanie testów i acceptance/surefire/failsafe.
- **Homebrew**: formula gotowa, ale wymaga pierwszego release'a z pre-built binarkami (aktualny tag v0.1.0). Po pushnięciu tagu np. v0.2.0, uruchomi się workflow który zbuduje binary i zaktualizuje formulę.
