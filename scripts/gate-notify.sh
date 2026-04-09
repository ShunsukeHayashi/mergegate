#!/bin/bash
# Polaris 完了通知: エージェント完了時にオーケストレーターに通知する
#
# Usage:
#   gate-notify.sh <event> <task-id> [details]
#
# Events: registered, assigned, merged, gate_rejected, error

set -euo pipefail

EVENT="${1:-}"
TASK_ID="${2:-}"
DETAILS="${3:-}"
MIYABI_BIN="$(git rev-parse --show-toplevel 2>/dev/null)/target/release/miyabi"
STORE="$(git rev-parse --show-toplevel 2>/dev/null)/project_memory/tasks.json"

if [ -z "$EVENT" ] || [ -z "$TASK_ID" ]; then
  echo "Usage: gate-notify.sh <event> <task-id> [details]"
  exit 1
fi

# タスク情報取得
TASK_TITLE=""
if [ -x "$MIYABI_BIN" ]; then
  TASK_TITLE=$("$MIYABI_BIN" gate --store-path "$STORE" --format json status "$TASK_ID" 2>/dev/null \
    | python3 -c "import sys,json; d=json.load(sys.stdin); t=[x for x in d.get('tasks',[]) if x['id']=='$TASK_ID']; print(t[0]['title'] if t else '')" 2>/dev/null || echo "")
fi

# 通知メッセージ構築
case "$EVENT" in
  registered)
    MSG="Polaris: タスク ${TASK_ID} を登録しました。${TASK_TITLE}"
    ;;
  assigned)
    MSG="Polaris: タスク ${TASK_ID} の作業を開始しました。${DETAILS}"
    ;;
  merged)
    MSG="Polaris: タスク ${TASK_ID} が完了しました。${TASK_TITLE}。後続タスクを確認してください。"
    ;;
  gate_rejected)
    MSG="Polaris: タスク ${TASK_ID} のゲートが拒否されました。理由: ${DETAILS}"
    ;;
  error)
    MSG="Polaris: エラー発生。タスク ${TASK_ID}。${DETAILS}"
    ;;
  *)
    MSG="Polaris: ${EVENT} — ${TASK_ID} ${DETAILS}"
    ;;
esac

# 1. 音声アナウンス
~/bin/announce "$MSG" 2>/dev/null &

# 2. 後続タスクの自動 dispatch 表示（merged の場合）
if [ "$EVENT" = "merged" ] && [ -x "$MIYABI_BIN" ]; then
  DISPATCHABLE=$("$MIYABI_BIN" gate --store-path "$STORE" dispatchable 2>/dev/null)
  if [ -n "$DISPATCHABLE" ]; then
    echo ""
    echo "📋 新たに実行可能になったタスク:"
    echo "$DISPATCHABLE"
    ~/bin/announce "後続タスクが実行可能になりました。" 2>/dev/null &
  fi
fi

# 3. Bus に記録
bash "$(git rev-parse --show-toplevel)/scripts/gate-bus-bridge.sh" complete "$TASK_ID" 2>/dev/null &

# 4. skill-bus に記録
npx miyabi bus record-run polaris \
  --agent "polaris" \
  --result "success" \
  --score 0.95 \
  --task "$EVENT: $TASK_ID $TASK_TITLE" 2>/dev/null &

wait
echo "$MSG"
