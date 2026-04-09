#!/bin/bash
# Claude Code PreToolUse Hook: ファイル編集前にロック確認
#
# Edit/Write ツールが呼ばれる前に実行される。
# ロックされていないファイルへの書き込みをブロックする。

TOOL_INPUT="$1"
MIYABI_BIN="/Users/shunsukehayashi/dev/platform/miyabi-cli-standalone/target/release/miyabi"
STORE="/Users/shunsukehayashi/dev/platform/miyabi-cli-standalone/project_memory/tasks.json"

# miyabi バイナリがなければスキップ
if [ ! -x "$MIYABI_BIN" ]; then
  exit 0
fi

# tasks.json がなければスキップ（初回）
if [ ! -f "$STORE" ]; then
  exit 0
fi

# 編集対象ファイルを抽出（file_path パラメータ）
TARGET_FILE=$(echo "$TOOL_INPUT" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('file_path', ''))
except:
    pass
" 2>/dev/null)

if [ -z "$TARGET_FILE" ]; then
  exit 0
fi

# リポルートからの相対パスに変換
REPO_ROOT="/Users/shunsukehayashi/dev/platform/miyabi-cli-standalone"
REL_PATH="${TARGET_FILE#$REPO_ROOT/}"

# ロック一覧を取得
LOCKS=$("$MIYABI_BIN" gate --store-path "$STORE" --format json locks 2>/dev/null)

# このファイルがロックされているか確認
LOCK_INFO=$(echo "$LOCKS" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    if isinstance(data, dict):
        for file, info in data.items():
            if file == '$REL_PATH' or '$REL_PATH'.endswith(file):
                print(f'{info.get(\"agent\",\"?\")}@{info.get(\"node\",\"?\")}')
                break
except:
    pass
" 2>/dev/null)

# ロックが存在し、かつ自分のロックでない場合はブロック
if [ -n "$LOCK_INFO" ]; then
  POLARIS_AGENT="${POLARIS_AGENT_ID:-claude}"
  if echo "$LOCK_INFO" | grep -qv "$POLARIS_AGENT"; then
    echo "⚠️ POLARIS: $REL_PATH は $LOCK_INFO がロック中です。"
    echo "miyabi gate assign でロックを取得してください。"
    # exit 2 でブロック（Claude Code はこれでツール実行を中止する）
    exit 2
  fi
fi

# ロックがない場合も警告（ただしブロックはしない）
ACTIVE_LOCKS=$("$MIYABI_BIN" gate --store-path "$STORE" locks 2>/dev/null | grep -c "->")
if [ "$ACTIVE_LOCKS" -gt 0 ]; then
  # 他のファイルにロックがある場合、このファイルにロックがないことを警告
  HAS_LOCK=$(echo "$LOCKS" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    if isinstance(data, dict) and '$REL_PATH' in data:
        print('yes')
except:
    pass
" 2>/dev/null)

  if [ -z "$HAS_LOCK" ]; then
    echo "ℹ️ POLARIS: $REL_PATH はロックされていません。miyabi gate assign で登録を推奨します。"
  fi
fi

exit 0
