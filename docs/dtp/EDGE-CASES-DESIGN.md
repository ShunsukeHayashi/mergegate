# エッジケース設計決定書

## #64: 並列 Codex のブランチ戦略

**決定**: ブランチを切る。ワークツリーは作らない。

- `miyabi gate assign` 時にエージェントが feature ブランチを作成
- ファイルロックで排他制御されるため、同じファイルの同時編集は CLI レベルで拒否
- ワークツリーのフルコピーは不要（Polaris のロック機構が論理的分離を提供）
- merge は PR 経由。main への直接コミットは pre-commit hook で拒否

## #68: マルチマシン分断時の二重ロック

**決定**: 現時点は単一マシン運用を前提。マルチマシンは段階的に対応。

- flock + CAS version check は同一マシン内で原子的に動作する（#67 で確認済み）
- マルチマシンの場合: `miyabi gate assign` 前に `git pull` を推奨
- tasks.json の CAS version が衝突を検出するため、分岐しても片方がエラーになる
- 将来: GitHub API を lock authority にする or Redis 共有 lock store

## #69: assign 後にロック外ファイルが変更必要になった場合

**決定**: attach_context でエージェントに影響範囲を事前通知。追加ロックは手動。

- attach_context (#58) が assign 時に自動実行される
- impact の depth1 シンボルがアタッチメントに含まれる
- エージェントは「他にどのファイルが影響を受けるか」を事前に把握できる
- 追加ファイルが必要になったら `miyabi gate assign` を再実行してロック拡張
- 将来: `--auto-lock-deps` で GNI depth=1 ファイルを自動ロック

## #70: 並列ブランチの merge による想定外の変更検知

**決定**: post-commit hook で GNI reindex + 通知。自動ブロックはしない。

- merge 後に gate-notify.sh が後続タスクの dispatchable を表示
- 将来: `npx gitnexus analyze` を merge 後に自動実行
- 他 implementing タスクの affected_files との交差チェック
- 交差があれば VOICEBOX + Telegram で警告（ブロックはしない）
