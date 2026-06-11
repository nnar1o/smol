#!/bin/bash
# Setup smol-smol multi-tenancy alias.
#
# Creates a symlink 'smol-smol' → /usr/local/bin/smol.
# When invoked as 'smol-smol', the binary reads SMOL_SMOL_TASKS_DIR
# or falls back to ~/.smol-smol/tasks — a fully separate namespace.
#
# Usage:
#   sudo bash scripts/setup-smol-smol.sh           # install symlink
#   bash scripts/setup-smol-smol.sh --dry-run      # show what would happen
#   bash scripts/setup-smol-smol.sh --target=~/.local/bin  # custom dir
#
# Example:
#   smol-smol --sync echo "hello from other tenant"
#   smol-smol list
#   SMOL_SMOL_TASKS_DIR=/tmp/smol-smol-tasks smol-smol status last

set -euo pipefail

DRY_RUN=false
TARGET_DIR="/usr/local/bin"

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        --target=*) TARGET_DIR="${arg#*=}" ;;
        --help|-h)
            echo "Usage: $0 [--dry-run] [--target=<dir>]"
            exit 0
            ;;
    esac
done

# Find smol binary
SMOL_BIN=""
for candidate in /usr/local/bin/smol ~/.cargo/bin/smol target/release/smol target/debug/smol; do
    if [ -x "$candidate" ]; then
        SMOL_BIN="$candidate"
        break
    fi
done

if [ -z "$SMOL_BIN" ]; then
    echo "Error: smol binary not found. Build it first with 'cargo build --release'"
    exit 1
fi

LINK_PATH="${TARGET_DIR}/smol-smol"

if [ "$DRY_RUN" = true ]; then
    echo "[dry-run] Would create symlink: ${LINK_PATH} -> ${SMOL_BIN}"
    echo "[dry-run] Then run: smol-smol --sync echo 'hello from smol-smol'"
    echo "[dry-run] See also: SMOL_SMOL_TASKS_DIR env var"
    exit 0
fi

# Create target directory if needed
mkdir -p "$TARGET_DIR"

# Remove existing link/file if present
if [ -e "$LINK_PATH" ] || [ -L "$LINK_PATH" ]; then
    rm -f "$LINK_PATH"
fi

ln -s "$SMOL_BIN" "$LINK_PATH"
echo "Created: ${LINK_PATH} -> ${SMOL_BIN}"
echo ""
echo "Test it:"
echo "  smol-smol --sync echo 'hello from smol-smol'"
echo "  smol-smol list"
echo ""
echo "The smol-smol instance uses its own directory:"
echo "  SMOL_SMOL_TASKS_DIR env var (or ~/.smol-smol/tasks)"
echo ""
echo "To see the difference, compare:"
echo "  smol   list  # tasks from ~/.smol/tasks"
echo "  smol-smol list  # tasks from ~/.smol-smol/tasks"
