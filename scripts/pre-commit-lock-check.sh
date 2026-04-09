#!/bin/bash
# Polaris: コミット前にファイルロックを検証する pre-commit hook
#
# ロックされているファイルを、ロック保持者以外が変更しようとしたら拒否する。

STORE="$(git rev-parse --show-toplevel)/project_memory/tasks.json"
MIYABI_BIN="$(git rev-parse --show-toplevel)/target/release/miyabi"

# miyabi バイナリがなければスキップ（ビルド前）
if [ ! -x "$MIYABI_BIN" ]; then
  exit 0
fi

# 変更されたファイル一覧
CHANGED_FILES=$(git diff --cached --name-only)

if [ -z "$CHANGED_FILES" ]; then
  exit 0
fi

# ロック一覧を取得
LOCKS=$("$MIYABI_BIN" gate --store-path "$STORE" --format json locks 2>/dev/null)

if [ -z "$LOCKS" ] || [ "$LOCKS" = "[]" ] || [ "$LOCKS" = "{}" ]; then
  exit 0  # ロックなし
fi

# 現在のエージェント ID（環境変数から取得、なければチェックしない）
AGENT_ID="${POLARIS_AGENT_ID:-}"

if [ -z "$AGENT_ID" ]; then
  exit 0  # エージェント ID 未設定ならチェックスキップ
fi

# 各変更ファイルのロック状態を確認
BLOCKED=0
for FILE in $CHANGED_FILES; do
  LOCK_OWNER=$(echo "$LOCKS" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    if isinstance(data, dict):
        entry = data.get('$FILE', {})
        if entry:
            print(entry.get('agent', '') + '@' + entry.get('node', ''))
except: pass
" 2>/dev/null)

  if [ -n "$LOCK_OWNER" ] && [ "$LOCK_OWNER" != "$AGENT_ID" ]; then
    echo "❌ BLOCKED: $FILE is locked by $LOCK_OWNER"
    echo "   You ($AGENT_ID) cannot modify this file."
    echo "   Run: miyabi gate assign <task-id> --files \"$FILE\" to acquire lock"
    BLOCKED=1
  fi
done

if [ "$BLOCKED" -eq 1 ]; then
  echo ""
  echo "🔒 Commit rejected by Polaris file lock enforcement."
  echo "   Set POLARIS_AGENT_ID to identify yourself."
  exit 1
fi

exit 0
