#!/bin/bash
# Polaris ↔ Bus ブリッジ
#
# miyabi gate の操作後に miyabi bus を自動呼び出しする。
# post-commit hook から呼ばれるか、直接呼び出す。
#
# Usage:
#   gate-bus-bridge.sh register <task-id> <title> [--issue N]
#   gate-bus-bridge.sh complete <task-id>
#   gate-bus-bridge.sh sync

set -euo pipefail

ACTION="${1:-}"
TASK_ID="${2:-}"
TITLE="${3:-}"
REPO="Miyabi-G-K/miyabi-cli-standalone"

case "$ACTION" in
  register)
    # miyabi gate register 後に Bus に自動 enqueue
    if [ -z "$TASK_ID" ] || [ -z "$TITLE" ]; then
      echo "Usage: gate-bus-bridge.sh register <task-id> <title>"
      exit 1
    fi

    # Issue 番号を抽出
    ISSUE_NUM=$(echo "$TASK_ID" | grep -oE '[0-9]+' | head -1)

    echo "[Bus Bridge] enqueue: $TASK_ID ($TITLE)"
    npx miyabi bus enqueue "$TITLE" \
      --agent "polaris" \
      --source "miyabi-gate" \
      --priority "medium" \
      ${ISSUE_NUM:+--issue "$ISSUE_NUM"} \
      ${REPO:+--repo "$REPO"} \
      2>/dev/null && echo "[Bus Bridge] ✅ enqueued" || echo "[Bus Bridge] ⚠️ enqueue failed (non-fatal)"
    ;;

  complete)
    # miyabi gate merge 後に Bus の該当タスクを complete
    if [ -z "$TASK_ID" ]; then
      echo "Usage: gate-bus-bridge.sh complete <task-id>"
      exit 1
    fi

    echo "[Bus Bridge] completing: $TASK_ID"

    # Bus の queue から該当タスクを探す
    PR_ID=$(npx miyabi bus dispatch --json 2>/dev/null \
      | python3 -c "
import sys,json
try:
    data = json.load(sys.stdin)
    for pr in data:
        if '$TASK_ID' in pr.get('task','') or '$TASK_ID' in pr.get('id',''):
            print(pr['id'])
            break
except: pass
" 2>/dev/null)

    if [ -n "$PR_ID" ]; then
      npx miyabi bus complete "$PR_ID" 2>/dev/null \
        && echo "[Bus Bridge] ✅ completed: $PR_ID" \
        || echo "[Bus Bridge] ⚠️ complete failed"
    else
      echo "[Bus Bridge] ℹ️ task not found in Bus queue (already completed or not enqueued)"
    fi
    ;;

  sync)
    # tasks.json の全タスクを Bus と同期
    echo "[Bus Bridge] syncing tasks.json → Bus"

    STORE="project_memory/tasks.json"
    if [ ! -f "$STORE" ]; then
      echo "[Bus Bridge] ⚠️ tasks.json not found"
      exit 1
    fi

    # pending タスクを Bus に enqueue
    python3 -c "
import json
with open('$STORE') as f:
    data = json.load(f)
for task in data.get('tasks', []):
    if task['current_state'] == 'pending':
        print(f\"{task['id']}|{task['title']}\")
" 2>/dev/null | while IFS='|' read -r id title; do
      echo "[Bus Bridge] enqueue: $id ($title)"
      npx miyabi bus enqueue "$title" \
        --agent "polaris" \
        --source "miyabi-gate-sync" \
        --priority "medium" \
        2>/dev/null || true
    done

    echo "[Bus Bridge] ✅ sync complete"
    ;;

  *)
    echo "Usage: gate-bus-bridge.sh <register|complete|sync> [args...]"
    exit 1
    ;;
esac
