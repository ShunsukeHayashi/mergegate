#!/bin/bash
# Polaris: コミット後に関連 Issue を自動更新する post-commit hook
#
# コミットメッセージに (#52, #53) のような Issue 番号があれば、
# 自動で gh issue comment を実行する。

REPO="Miyabi-G-K/miyabi-cli-standalone"

# 直前のコミット情報を取得
COMMIT_HASH=$(git rev-parse --short HEAD)
COMMIT_MSG=$(git log -1 --pretty=%B)
AUTHOR=$(git log -1 --pretty=%an)

# コミットメッセージから Issue 番号を抽出 (#52, #53 等)
ISSUE_NUMS=$(echo "$COMMIT_MSG" | grep -oE '#[0-9]+' | sed 's/#//')

if [ -z "$ISSUE_NUMS" ]; then
  exit 0  # Issue 番号がなければ何もしない
fi

# 各 Issue にコメントを投稿
for NUM in $ISSUE_NUMS; do
  gh issue comment "$NUM" \
    --repo "$REPO" \
    --body "✅ **コミット反映**: \`$COMMIT_HASH\` by $AUTHOR

\`\`\`
$COMMIT_MSG
\`\`\`

cargo test: $(cargo test 2>&1 | grep 'test result' | tail -1)" \
    2>/dev/null &
done

# バックグラウンドで実行（コミットをブロックしない）
wait
