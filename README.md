# smol

Smart Minimal Output Logger — run commands and get summarized output for LLMs.

## Install

### Linux (x86_64 / ARM64) & macOS (Intel / Apple Silicon)

Detects your OS and architecture automatically:

```sh
curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-$(uname -m | sed 's/x86_64/x86_64/; s/aarch64/aarch64/; s/arm64/aarch64/')-$(uname -s | tr '[:upper:]' '[:lower:]' | sed 's/darwin/apple-darwin/; s/linux/unknown-linux-gnu/').tar.gz | tar xz -C /usr/local/bin
```

Or pick your platform manually:

| Platform | Command |
|----------|---------|
| Linux x86_64 | `curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-unknown-linux-gnu.tar.gz \| tar xz -C /usr/local/bin` |
| Linux ARM64 | `curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-aarch64-unknown-linux-gnu.tar.gz \| tar xz -C /usr/local/bin` |
| macOS Intel | `curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-apple-darwin.tar.gz \| tar xz -C /usr/local/bin` |
| macOS Apple Silicon | `curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-aarch64-apple-darwin.tar.gz \| tar xz -C /usr/local/bin` |

### Windows (x86_64)

```powershell
curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-pc-windows-msvc.zip -o smol.zip
Expand-Archive smol.zip -DestinationPath . ; del smol.zip
```

### From source

```sh
cargo install --git https://github.com/nnar1o/smol
```

## Usage

```sh
smol <command> [args...]
```
