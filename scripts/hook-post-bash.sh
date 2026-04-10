#!/bin/bash
# Claude Code PostToolUse Hook: Bash 実行後にロック状態をリマインド
#
# git commit が実行された場合、post-commit hook が発火するので
# ここでは軽い状態表示のみ。

TOOL_INPUT="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MIYABI_BIN="${MIYABI_BIN:-$REPO_ROOT/target/release/miyabi}"
STORE="${POLARIS_TASK_STORE:-$REPO_ROOT/project_memory/tasks.json}"

# miyabi バイナリがなければスキップ
if [ ! -x "$MIYABI_BIN" ]; then
  exit 0
fi

# git commit を検出した場合にステータス表示
if echo "$TOOL_INPUT" | grep -q "git commit"; then
  ACTIVE=$("$MIYABI_BIN" gate --store-path "$STORE" locks 2>/dev/null | grep -c "->")
  if [ "$ACTIVE" -gt 0 ]; then
    echo "ℹ️ POLARIS: $ACTIVE 件のファイルがまだロック中です。作業完了したら miyabi gate merge または miyabi gate manual-complete を実行してください。"
  fi
fi

exit 0
