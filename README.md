# smol

Smart Minimal Output Logger — run commands and get summarized output for LLMs.

## Install

### Linux (x86_64 / ARM64) & macOS (Intel / Apple Silicon)

Detects your OS and architecture automatically:

```sh
mkdir -p ~/.local/bin && curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-$(uname -m | sed 's/x86_64/x86_64/; s/aarch64/aarch64/; s/arm64/aarch64/')-$(uname -s | tr '[:upper:]' '[:lower:]' | sed 's/darwin/apple-darwin/; s/linux/unknown-linux-gnu/').tar.gz | tar xz -C ~/.local/bin
```

Make sure `~/.local/bin` is in your `PATH`. If not, add this to your shell config (`.bashrc`, `.zshrc`, etc.):

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Or pick your platform manually:

| Platform | Command |
|----------|---------|
| Linux x86_64 | `mkdir -p ~/.local/bin && curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-unknown-linux-gnu.tar.gz \| tar xz -C ~/.local/bin` |
| Linux ARM64 | `mkdir -p ~/.local/bin && curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-aarch64-unknown-linux-gnu.tar.gz \| tar xz -C ~/.local/bin` |
| macOS Intel | `mkdir -p ~/.local/bin && curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-apple-darwin.tar.gz \| tar xz -C ~/.local/bin` |
| macOS Apple Silicon | `mkdir -p ~/.local/bin && curl -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-aarch64-apple-darwin.tar.gz \| tar xz -C ~/.local/bin` |

### Windows (x86_64)

```powershell
$dir = "$env:USERPROFILE\.local\bin"; mkdir $dir -Force | Out-Null; curl.exe -fsSL https://github.com/nnar1o/smol/releases/latest/download/smol-x86_64-pc-windows-msvc.zip -o "$env:TEMP\smol.zip"; Expand-Archive "$env:TEMP\smol.zip" -DestinationPath $dir; del "$env:TEMP\smol.zip"
```

Add `%USERPROFILE%\.local\bin` to your `PATH` (System Properties → Environment Variables).

### Homebrew

```sh
brew install nnar1o/tap/smol
```

### From source

```sh
cargo install --git https://github.com/nnar1o/smol
```

## Usage

```sh
smol <command> [args...]
```
