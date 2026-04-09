# Handoff Protocol — エージェント間引き継ぎ手順

> 全 Phase のエージェント交代時に必ず従うプロシージャ。
> 引き継ぎ条件を満たさない限り、次のエージェントは作業を開始してはならない。

---

## 1. 引き継ぎの原則

```
前任エージェント（sender）
    │
    ├─ 1. GATE.md の全条件を GREEN にする
    ├─ 2. HANDOFF_NOTE.md を書く
    ├─ 3. git commit + push する
    └─ 4. 完了を通知する
              │
              ▼
後任エージェント（receiver）
    │
    ├─ 1. git pull で最新を取得
    ├─ 2. HANDOFF_NOTE.md を読む
    ├─ 3. cargo test + cargo clippy で GREEN を確認
    ├─ 4. TASKS.md の未完了チェックボックスを確認
    └─ 5. 作業開始
```

---

## 2. HANDOFF_NOTE.md テンプレート

各 Phase ディレクトリに sender が書き残すファイル。

```markdown
# Handoff Note: Phase N

## Sender
- Agent: {agent name}@{node}
- Completed at: {ISO 8601 timestamp}
- Commit: {git commit hash}

## Status
- cargo test: {PASS/FAIL} ({N}/{M} tests)
- cargo clippy: {PASS/FAIL}
- GATE.md: {ALL GREEN / N remaining}

## What was done
- {completed task 1}
- {completed task 2}

## What was NOT done (and why)
- {skipped task}: {reason}

## Known issues / warnings
- {issue 1}
- {issue 2}

## Context for receiver
- {important context that isn't obvious from code}
- {files that were tricky or need special attention}

## How to verify
```bash
cargo test
cargo clippy --all-targets -- -D warnings
```
```

---

## 3. エージェント別引き継ぎプロシージャ

### 3.1 Codex → Codex（worktree 間）

```
Codex A (sender):
  1. cargo test → GREEN
  2. cargo clippy → GREEN
  3. Write phase-N/HANDOFF_NOTE.md
  4. git add -A && git commit -m "[完了] Phase N: {summary}"
  5. git push

Codex B (receiver):
  1. git pull origin main
  2. Read phase-N/HANDOFF_NOTE.md
  3. Read phase-N+1/TASKS.md
  4. Read phase-N+1/ASSIGNMENT.md
  5. cargo test → must be GREEN (sender's work didn't break anything)
  6. Begin phase-N+1/TASKS.md checkboxes
```

### 3.2 Codex → Claude Code（worktree → local）

```
Codex (sender):
  1. cargo test → GREEN
  2. Write phase-N/HANDOFF_NOTE.md
  3. git commit + push

Claude Code (receiver):
  1. git pull
  2. Read HANDOFF_NOTE.md
  3. cargo test (verify locally)
  4. Read phase-N+1/TASKS.md
  5. If Phase requires interactive debugging (5, 6, 8): proceed locally
  6. If Phase is parallelizable: dispatch sub-tasks to Codex
```

### 3.3 Claude Code → Codex（local → worktree）

```
Claude Code (sender):
  1. cargo test → GREEN
  2. Write phase-N/HANDOFF_NOTE.md
  3. git commit + push
  4. Dispatch via tmux:
     tmux send-keys -t %{pane} "codex 'Execute Phase {N+1}. \
       Read docs/autorun/phase-{N+1}-*/TASKS.md and HANDOFF from phase-{N}. \
       git pull first. Run cargo test before starting. \
       Write your HANDOFF_NOTE.md when done.'" Enter

Codex (receiver):
  1. git pull
  2. Read phase-N/HANDOFF_NOTE.md (predecessor context)
  3. cargo test → GREEN
  4. Execute TASKS.md
  5. Write phase-N+1/HANDOFF_NOTE.md
  6. git commit + push
```

### 3.4 Claude Code → OpenClaw main（local → gateway）

```
Claude Code (sender):
  1. cargo test → GREEN
  2. Write phase-N/HANDOFF_NOTE.md
  3. git commit + push
  4. Dispatch:
     openclaw agent --agent main --message \
       "[TASK] Execute Phase {N+1} of DTP. \
        Repo: Miyabi-G-K/deterministic-task-protocol. \
        Read docs/autorun/phase-{N+1}-*/TASKS.md. \
        GATE: docs/autorun/phase-{N+1}-*/GATE.md. \
        Previous handoff: docs/autorun/phase-{N}-*/HANDOFF_NOTE.md."

OpenClaw main (receiver):
  1. git clone / pull
  2. Read HANDOFF_NOTE.md
  3. cargo test → GREEN
  4. Execute or delegate to sub-agents
  5. Write HANDOFF_NOTE.md
  6. git commit + push
  7. Report completion via Telegram
```

---

## 4. 引き継ぎ条件チェックリスト（sender 用）

sender は以下を全て満たしてから引き継ぐ:

```
[ ] cargo test → GREEN
[ ] cargo clippy -- -D warnings → GREEN
[ ] GATE.md の全条件が GREEN
[ ] HANDOFF_NOTE.md を該当 Phase ディレクトリに書いた
[ ] git add + commit + push 済み
[ ] 次の Phase の ASSIGNMENT.md を確認し、receiver を特定した
[ ] receiver への通知を送った（tmux / OpenClaw / Telegram）
```

---

## 5. 引き継ぎ受理チェックリスト（receiver 用）

receiver は以下を全て満たしてから作業開始:

```
[ ] git pull で最新コードを取得した
[ ] HANDOFF_NOTE.md を読んだ
[ ] cargo test → GREEN（sender の作業が壊れていないことを確認）
[ ] TASKS.md の残チェックボックスを確認した
[ ] GATE.md の承認条件を理解した
[ ] ASSIGNMENT.md の制約（並列可否、依存）を理解した
```

---

## 6. 引き継ぎ失敗時のリカバリ

### sender の GATE が GREEN にならない場合

```
sender:
  1. HANDOFF_NOTE.md に "Status: PARTIAL" と書く
  2. What was NOT done に未完了項目を列挙
  3. Known issues に失敗原因を書く
  4. git commit + push（作業途中でも push）
  5. 人間に ESCALATE:
     "Phase N が GATE 未達。理由: {reason}。対応方針を判断してください。"
```

### receiver が cargo test FAIL を検出した場合

```
receiver:
  1. HANDOFF_NOTE.md を再確認（sender が認識していた問題か？）
  2. git log で sender の最終コミットを確認
  3. 自分のコードが原因でないことを確認
  4. sender に差し戻し（tmux / OpenClaw で通知）:
     "Phase N の HANDOFF を受理できません。cargo test FAIL。
      失敗テスト: {test name}。sender 側で修正してください。"
  5. 3回差し戻しで人間 ESCALATE
```

### エージェントがクラッシュ/応答なしの場合

```
監視側（Claude Code or 人間）:
  1. tmux capture-pane で最後の出力を確認
  2. git log で最終コミットを確認
  3. cargo test で現在の状態を確認
  4. 代替エージェントを起動:
     - 同じ Phase の TASKS.md を渡す
     - HANDOFF_NOTE.md がなければ「前任クラッシュ」として扱う
     - cargo test GREEN から再開
```

---

## 7. 並列 Phase の同期点

Phase 2 と Phase 3 は並列実行可能だが、Phase 6 は両方の完了を待つ。

```
Phase 2 (Codex B) ─────┐
                        ├──→ 同期点: 両方の GATE GREEN を確認
Phase 3 (Codex C) ─────┘        │
                                 ▼
                          Phase 4 (Phase 3 依存)
                                 │
                          Phase 5 (Phase 3 依存)
                                 │
                          Phase 6 開始条件:
                            [ ] Phase 2 GATE GREEN
                            [ ] Phase 3 GATE GREEN
                            [ ] Phase 4 GATE GREEN
                            [ ] Phase 5 GATE GREEN
                            [ ] 全 HANDOFF_NOTE.md 存在
```

同期点の確認は **Claude Code（オーケストレーター）** が行う:

```bash
# 全 Phase の GATE 状態を一括確認
for phase in phase-2-state-machine phase-3-event-store phase-4-file-lock phase-5-github-sync; do
  echo "=== $phase ==="
  cat docs/autorun/$phase/GATE.md | grep "\[x\]" | wc -l
  cat docs/autorun/$phase/GATE.md | grep "\[ \]" | wc -l
done
```

---

## 8. 通知テンプレート

### Phase 完了通知（sender → 全体）

```
[DTP] Phase {N} 完了
Agent: {agent}@{node}
Commit: {hash}
Tests: {N}/{M} GREEN
GATE: ALL GREEN
Next: Phase {N+1} → {receiver agent}
HANDOFF: docs/autorun/phase-{N}-*/HANDOFF_NOTE.md
```

### Phase 開始通知（receiver → 全体）

```
[DTP] Phase {N} 開始
Agent: {agent}@{node}
Predecessor: Phase {N-1} by {sender agent}
HANDOFF received: ✅
cargo test: GREEN
Starting TASKS.md execution...
```

### エスカレーション通知

```
🚨 [DTP] Phase {N} ESCALATION
Agent: {agent}@{node}
Issue: {description}
Attempts: {N}/3
Action needed: 人間判断
HANDOFF: docs/autorun/phase-{N}-*/HANDOFF_NOTE.md
```
