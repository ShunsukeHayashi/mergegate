#!/bin/bash
# miyabi gni ask — 自然言語で GNI に質問して自然言語で返す
#
# Usage:
#   miyabi gni ask "このリポの構造を教えて"
#   miyabi gni ask "protocol.rs の依存関係は？"
#   miyabi gni ask "テストはいくつある？"

set -euo pipefail

REPO="miyabi-cli-standalone"
QUESTION="${1:-}"

if [ -z "$QUESTION" ]; then
  echo "使い方: miyabi gni ask \"質問\""
  echo ""
  echo "例:"
  echo "  miyabi gni ask \"このリポの構造を教えて\""
  echo "  miyabi gni ask \"protocol.rs は何に依存してる？\""
  echo "  miyabi gni ask \"DTP の関数一覧\""
  echo "  miyabi gni ask \"ロック関連のフロー\""
  echo "  miyabi gni ask \"一番大きいクラスターは？\""
  exit 0
fi

Q=$(echo "$QUESTION" | tr '[:upper:]' '[:lower:]')

# キーワードで分類して適切なクエリを実行
if echo "$Q" | grep -qE "構造|概要|全体|overview"; then
  echo "このリポジトリの概要です。"
  echo ""
  npx gitnexus status 2>/dev/null
  echo ""
  echo "主要なクラスター（機能領域）:"
  npx gitnexus cypher --repo $REPO \
    "MATCH (c:Community) WHERE c.symbolCount > 15 RETURN c.label AS name, c.symbolCount AS size ORDER BY size DESC LIMIT 10" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
for row in data.get('rows',json.loads(data.get('markdown','[]').replace('|','\t').strip()) if 'markdown' in data else []):
    pass
# fallback: parse markdown table
import re
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            print(f'  {cols[0]}: {cols[1]}シンボル')
" 2>/dev/null

elif echo "$Q" | grep -qE "依存|depend|使って|呼んで|import"; then
  # ファイル名を抽出
  FILE=$(echo "$QUESTION" | grep -oE '[a-z_]+\.rs' | head -1)
  if [ -z "$FILE" ]; then
    echo "どのファイルの依存関係を見ますか？ファイル名を含めて質問してください。"
    exit 1
  fi
  echo "${FILE} が依存しているモジュール:"
  echo ""
  npx gitnexus cypher --repo $REPO \
    "MATCH (caller:Function)-[r]->(callee:Function) WHERE caller.filePath CONTAINS '$FILE' AND NOT callee.filePath CONTAINS '$FILE' RETURN DISTINCT callee.filePath AS module, count(*) AS calls ORDER BY calls DESC LIMIT 10" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            print(f'  → {cols[0]} ({cols[1]}箇所から呼び出し)')
else:
    print('  依存関係が見つかりませんでした。')
" 2>/dev/null

elif echo "$Q" | grep -qE "関数|function|一覧|list|api"; then
  FILE=$(echo "$QUESTION" | grep -oE '[a-z_]+\.rs' | head -1)
  FILTER=""
  if [ -n "$FILE" ]; then
    FILTER="WHERE f.filePath CONTAINS '$FILE'"
    echo "${FILE} の関数一覧:"
  else
    FILTER="WHERE f.filePath CONTAINS 'gate.rs' OR f.filePath CONTAINS 'lock.rs' OR f.filePath CONTAINS 'store.rs' OR f.filePath CONTAINS 'protocol.rs'"
    echo "DTP モジュールの関数一覧:"
  fi
  echo ""
  npx gitnexus cypher --repo $REPO \
    "MATCH (f:Function) $FILTER RETURN f.name AS name, f.filePath AS file ORDER BY f.filePath, f.name LIMIT 30" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
current_file = ''
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            f = cols[1].split('/')[-1]
            if f != current_file:
                current_file = f
                print(f'\n  [{f}]')
            print(f'    {cols[0]}()')
" 2>/dev/null

elif echo "$Q" | grep -qE "フロー|flow|実行|process|パス"; then
  KEYWORD=$(echo "$QUESTION" | grep -oE '[A-Za-z_]+' | head -1)
  echo "実行フロー（${KEYWORD:-全体}）:"
  echo ""
  if [ -n "$KEYWORD" ]; then
    FILTER="WHERE p.label CONTAINS '$KEYWORD'"
  else
    FILTER=""
  fi
  npx gitnexus cypher --repo $REPO \
    "MATCH (p:Process) $FILTER RETURN p.label AS flow, p.stepCount AS steps ORDER BY p.stepCount DESC LIMIT 10" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            print(f'  {cols[0]} ({cols[1]}ステップ)')
else:
    print('  該当するフローが見つかりませんでした。')
" 2>/dev/null

elif echo "$Q" | grep -qE "クラスター|cluster|領域|area|モジュール"; then
  echo "機能クラスター（シンボル数順）:"
  echo ""
  npx gitnexus cypher --repo $REPO \
    "MATCH (c:Community) WHERE c.symbolCount > 5 RETURN c.label AS name, c.symbolCount AS size ORDER BY size DESC LIMIT 15" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for i,line in enumerate(lines[1:],1):
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            bar = '█' * min(int(int(cols[1])/3), 20)
            print(f'  {i:2d}. {cols[0]:30s} {cols[1]:>3s} {bar}')
" 2>/dev/null

elif echo "$Q" | grep -qE "テスト|test|カバレッジ|coverage"; then
  echo "テスト状況:"
  echo ""
  cargo test --all 2>&1 | grep "test result" | while read line; do
    echo "  $line"
  done
  echo ""
  echo "DTP モジュール別テスト数:"
  for f in gate.rs lock.rs store.rs protocol.rs; do
    count=$(grep -c "#\[test\]" crates/mergegate-core/src/$f 2>/dev/null || echo "0")
    echo "  $f: ${count}件"
  done

elif echo "$Q" | grep -qE "ロック|lock|競合|conflict"; then
  echo "現在のファイルロック状態:"
  echo ""
  target/release/miyabi gate locks 2>/dev/null || echo "  (miyabi バイナリが見つかりません)"
  echo ""
  echo "ロック関連の実行フロー:"
  npx gitnexus cypher --repo $REPO \
    "MATCH (p:Process) WHERE p.label CONTAINS 'Lock' OR p.label CONTAINS 'lock' OR p.label CONTAINS 'Acquire' OR p.label CONTAINS 'Release' RETURN p.label AS flow, p.stepCount AS steps ORDER BY p.stepCount DESC LIMIT 5" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if len(cols)>=2:
            print(f'  {cols[0]} ({cols[1]}ステップ)')
" 2>/dev/null

elif echo "$Q" | grep -qE "ファイル|file|探|search|どこ"; then
  KEYWORD=$(echo "$QUESTION" | grep -oE '[a-zA-Z_.-]+\.[a-z]+' | head -1)
  if [ -z "$KEYWORD" ]; then
    KEYWORD=$(echo "$QUESTION" | sed 's/.*[「」]\(.*\)[「」].*/\1/' | head -1)
  fi
  echo "「${KEYWORD}」を含むファイル:"
  echo ""
  npx gitnexus cypher --repo $REPO \
    "MATCH (f:File) WHERE f.filePath CONTAINS '$KEYWORD' RETURN f.filePath ORDER BY f.filePath LIMIT 20" 2>/dev/null \
    | python3 -c "
import sys,json
data=json.loads(sys.stdin.read())
md = data.get('markdown','')
lines = [l.strip() for l in md.split('\n') if l.strip() and not l.strip().startswith('---')]
if len(lines)>1:
    for line in lines[1:]:
        cols = [c.strip() for c in line.split('|') if c.strip()]
        if cols:
            print(f'  {cols[0]}')
else:
    print('  見つかりませんでした。')
" 2>/dev/null

else
  echo "すみません、質問の意図を特定できませんでした。"
  echo ""
  echo "以下のような質問ができます:"
  echo "  「このリポの構造を教えて」"
  echo "  「protocol.rs は何に依存してる？」"
  echo "  「DTP の関数一覧」"
  echo "  「ロック関連のフロー」"
  echo "  「一番大きいクラスターは？」"
  echo "  「テストはいくつある？」"
  echo "  「gate.rs を探して」"
fi
