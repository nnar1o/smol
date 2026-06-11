#!/bin/bash
# smol hook for Claude Code
# Wraps Bash commands with smol
if [ "$TOOL_NAME" = "Bash" ]; then
  COMMAND=$(echo "$TOOL_INPUT" | jq -r '.command // empty')
  if [ -n "$COMMAND" ]; then
    case "$COMMAND" in
      smol\ *|cd\ *|exit*|export\ *) ;;
      *)
        if [ ${#COMMAND} -gt 10 ]; then
          echo "$TOOL_INPUT" | jq --arg cmd "smol --sync $COMMAND" '.command = $cmd'
          exit 0
        fi
        ;;
    esac
  fi
fi
echo "$TOOL_INPUT"
