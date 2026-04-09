# Phase C 修正計画 — GATE の甘い部分を全て潰す

> 検証で発見した 6 件の不具合を修正する

---

## 修正 1: GATE 0 — Issue=0 で登録拒否

**現状**: `register --title "test"` が issue=0 でも通る
**期待**: issue > 0 でなければ登録拒否

**修正箇所**: `crates/miyabi-core/src/protocol.rs` の register 処理
```rust
if task.issue == 0 {
    return Err(ProtocolError::Gate("issue number must be > 0"));
}
```

**テスト**:
- [ ] `register --title "test"` → exit 1 (issue 未指定)
- [ ] `register --issue 0 --title "test"` → exit 1
- [ ] `register --issue 1 --title "test"` → exit 0

---

## 修正 2: GATE 5 — ブランチ名バリデーション

**現状**: `branch task-1 bad-name` が通る
**期待**: `feature/issue-N-*` or `fix/issue-N-*` or `hotfix/issue-N-*` のみ許可

**修正箇所**: `crates/miyabi-core/src/gate.rs` の branch 検証
```rust
pub fn validate_branch_name(name: &str) -> Result<(), GateError> {
    let valid = name.starts_with("feature/issue-")
        || name.starts_with("fix/issue-")
        || name.starts_with("hotfix/issue-");
    if !valid {
        return Err(GateError::InvalidBranchName(name.to_string()));
    }
    Ok(())
}
```

**テスト**:
- [ ] `branch task-1 bad-name` → exit 1
- [ ] `branch task-1 feature/issue-1-test` → exit 0
- [ ] `branch task-1 fix/issue-1-bugfix` → exit 0
- [ ] `branch task-1 main` → exit 1

---

## 修正 3: GATE 3 — HIGH risk 承認チェック

**現状**: `impact task-1 --risk high` の後、承認なしで assign できる
**期待**: HIGH/CRITICAL は `--approve` フラグなしで assign 拒否

**修正箇所**: `crates/miyabi-core/src/protocol.rs` の assign 処理
```rust
if let Some(impact) = &task.impact {
    if matches!(impact.risk_level, RiskLevel::HIGH | RiskLevel::CRITICAL) {
        if task.human_approval.is_none() {
            return Err(ProtocolError::Gate("HIGH/CRITICAL risk requires --approve"));
        }
    }
}
```

**修正箇所**: `crates/miyabi-core/src/protocol.rs` の impact 処理
- `--approve` フラグが true なら `human_approval` を記録

**テスト**:
- [ ] `impact task-1 --risk high` → 記録成功
- [ ] `assign task-1 ...` → exit 1 (承認なし)
- [ ] `impact task-1 --risk high --approve` → 記録 + 承認
- [ ] `assign task-1 ...` → exit 0 (承認済み)
- [ ] `impact task-1 --risk low` → `assign` そのまま OK

---

## 修正 4: GATE 7 — merge 後のロック自動解放

**現状**: merge 後も `locks` にファイルが残る
**期待**: merge 成功時にロック自動解放 + 後続タスクの依存解除

**修正箇所**: `crates/miyabi-core/src/protocol.rs` の merge 処理
```rust
// merge 成功後
self.lock_manager.release_lock(task_id, &snapshot_store)?;
// 後続タスクの依存チェック
for dependent_id in &task.dependents {
    // dependent の state が blocked なら pending に戻す
}
```

**テスト**:
- [ ] register A → assign A (lock src/a.rs) → merge A → `locks` が空
- [ ] register A, B(dep=A) → merge A → B が dispatchable に出現

---

## 修正 5: exit code 分類

**現状**: 不正 SHA が exit 2 (input_error)
**期待**: GATE 拒否は exit 1、入力フォーマットエラーは exit 2

**修正箇所**: `crates/miyabi-cli/src/main.rs` の exit code マッピング
```rust
match result {
    Ok(_) => ExitCode::from(0),
    Err(ProtocolError::Gate(_)) => ExitCode::from(1),      // GATE 拒否
    Err(ProtocolError::Lock(_)) => ExitCode::from(1),      // ロック競合
    Err(ProtocolError::DependencyBlocked(_)) => ExitCode::from(1), // 依存ブロック
    Err(_) => ExitCode::from(2),                            // その他入力エラー
}
```

**テスト**:
- [ ] 不正 SHA → exit 1 (GATE 拒否)
- [ ] ロック競合 → exit 1
- [ ] 依存ブロック → exit 1
- [ ] 不明タスク ID → exit 2 (入力エラー)

---

## 修正 6: 依存ブロック時の exit code

**現状**: 依存未解決で assign → exit 0 (成功扱い)
**期待**: exit 1 (GATE 拒否)

**修正箇所**: `crates/miyabi-core/src/protocol.rs` の assign 処理
- 依存チェックを assign の冒頭で実行
- 未解決なら `ProtocolError::DependencyBlocked` を返す

**テスト**:
- [ ] register A, B(dep=A) → assign B → exit 1 (A が done じゃない)
- [ ] merge A → assign B → exit 0

---

## 実行順序

```
修正 1 (issue=0 拒否)     ← 独立、最初にやる
修正 2 (ブランチ名)       ← 独立、並行可
修正 5 (exit code)        ← 独立、並行可
修正 6 (依存 exit code)   ← 修正 5 と同時にやる
修正 3 (HIGH 承認)        ← protocol.rs の変更、修正 1 の後
修正 4 (ロック解放)       ← protocol.rs の変更、修正 3 の後
```

## 工数

| 修正 | 変更行数 | テスト行数 |
|------|---------|-----------|
| 1 | ~5 | ~10 |
| 2 | ~10 | ~15 |
| 3 | ~20 | ~20 |
| 4 | ~15 | ~20 |
| 5 | ~10 | ~15 |
| 6 | ~10 | ~10 |
| **合計** | **~70** | **~90** |

**推定時間: 15〜20分（Codex 1体）**

## 承認ゲート

全修正完了後:
- [ ] `cargo test --all` → GREEN
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → ゼロ
- [ ] テスト 1: issue=0 → exit 1
- [ ] テスト 2: bad-branch → exit 1
- [ ] テスト 3: HIGH + 承認なし → exit 1
- [ ] テスト 4: merge 後 locks → 空
- [ ] テスト 5: 不正 SHA → exit 1
- [ ] テスト 6: 依存ブロック → exit 1
- [ ] E2E: register → done の全シーケンス（全 GATE 通過）
