# ロールバック・中止ポイント定義

> 各 Phase の「ここまで戻れる」「ここで止める」の判断基準。
> エージェントはこのルールに従い、人間の承認なしにロールバックを実行してよい。

---

## 1. ロールバックポイント一覧

各 Phase の開始コミットが安全なロールバック先。

| Phase | ロールバック先 | git tag | 条件 |
|-------|-------------|---------|------|
| 0 | 初期コミット | `v0.0-initial` | いつでも戻れる |
| 1 | Phase 0 完了時点 | `v0.1-phase0-done` | Phase 1 の型変更が既存テストを壊した場合 |
| 2 | Phase 1 完了時点 | `v0.2-phase1-done` | ステートマシン改造が壊れた場合 |
| 3 | Phase 1 完了時点 | `v0.2-phase1-done` | Event Store 実装が壊れた場合（Phase 2 とは独立） |
| 4 | Phase 3 完了時点 | `v0.4-phase3-done` | ロック実装が壊れた場合 |
| 5 | Phase 3 完了時点 | `v0.4-phase3-done` | GitHub 同期が壊れた場合 |
| 6 | Phase 5 完了時点 | `v0.6-phase5-done` | Protocol 統合が壊れた場合 |
| 7 | Phase 6 完了時点 | `v0.7-phase6-done` | CLI 実装が壊れた場合 |
| 8 | Phase 7 完了時点 | `v0.8-phase7-done` | E2E が壊れた場合 |

### タグの打ち方

各 Phase の GATE GREEN 確認後、次の Phase に進む前に必ずタグを打つ:

```bash
git tag v0.{N+1}-phase{N}-done
git push origin v0.{N+1}-phase{N}-done
```

### ロールバックの実行方法

```bash
# Phase N が壊れた → Phase N-1 の完了地点に戻す
git log --oneline --all  # タグを確認
git checkout v0.{N}-phase{N-1}-done  # 安全地点に移動
git checkout -b rollback/phase-{N}-retry  # リトライ用ブランチ作成
# 修正後、main にマージ
```

---

## 2. 中止ポイント（ここで止める判断基準）

### Phase レベルの中止条件

| 条件 | アクション |
|------|-----------|
| 同じテストが **3回連続失敗** | Phase を中止。人間にエスカレーション |
| `cargo build` が通らない状態が **30分以上継続** | Phase を中止。ロールバック |
| 依存クレートのバグで進めない | Phase を中止。Issue 作成して待機 |
| 設計上の根本的な問題が発覚 | **全 Phase 中止**。設計レビューに戻る |

### プロジェクトレベルの中止条件

| 条件 | アクション |
|------|-----------|
| Codex レビューで **Critical 指摘が未解決のまま Phase 6 に到達** | 全停止。指摘を解決してから再開 |
| GitHub API の仕様変更で Phase 5 が成立しない | Phase 5 を再設計。Phase 6 以降は凍結 |
| Rust コンパイラのバグで unsafe_code = "forbid" と競合 | バグ報告 + 回避策を検討 |
| 人間が「止めろ」と言った | **即座に全停止**。コミット + push して待機 |

---

## 3. 各 Phase の安全な中断方法

### 作業途中で中断する場合

```bash
# 1. 現在の作業をコミット（RED でもよい）
git add -A
git commit -m "[中断] Phase {N}: {理由}"

# 2. push して他エージェントと共有
git push

# 3. HANDOFF_NOTE.md に「中断」と書く
cat > autorun/phase-{N}-*/HANDOFF_NOTE.md << 'EOF'
# Handoff Note: Phase N — 中断

## Status: 中断
- Reason: {理由}
- Tests: {状態}
- Last working commit: {hash}

## 再開手順
1. git pull
2. cargo test で現状確認
3. {具体的な再開ポイント}
EOF

# 4. push
git add -A && git commit -m "[中断] Phase {N}: HANDOFF_NOTE 追記" && git push
```

### ロールバックして再開する場合

```bash
# 1. 安全地点を確認
git log --oneline --tags

# 2. ロールバック（ブランチで）
git checkout -b rollback/phase-{N}-attempt-{M} v0.{N}-phase{N-1}-done

# 3. 壊れた変更を選択的に取り込み（cherry-pick）
git cherry-pick {良いコミットだけ}

# 4. テスト確認
cargo test && cargo clippy --all-targets -- -D warnings

# 5. main に戻す
git checkout main
git merge rollback/phase-{N}-attempt-{M}
git push
```

---

## 4. 復旧不能な場合の最終手段

全てのリトライが失敗し、ロールバックしても直らない場合:

```
1. git tag v0.{N}-abandoned-{date} で現状を記録
2. 人間に以下を報告:
   - 何が壊れたか
   - どのコミットまでは動いていたか
   - 試したリトライの内容と結果
   - 推奨アクション（設計変更 / 依存変更 / スコープ縮小）
3. 人間の判断を待つ。判断が出るまで一切のコミットをしない
```

---

## 5. 安全境界の定義

| 操作 | 安全？ | 備考 |
|------|--------|------|
| `git tag` を打つ | 安全 | いつでも打ってよい |
| `git checkout` でタグに移動 | 安全 | detached HEAD になるが問題なし |
| `git branch` でブランチ作成 | 安全 | |
| `git cherry-pick` | 注意 | コンフリクト時は手動解決 |
| `git merge` | 注意 | テスト GREEN を確認してからマージ |
| `git revert` | 安全 | 特定コミットを取り消す場合に使用 |
| `git reset --hard` | **禁止** | GIT-RULES.md 参照 |
| `git push --force` | **禁止** | GIT-RULES.md 参照 |
