# MergeGate — 分散エージェント実行・記憶モデル設計ビジョン

_林 駿甫 | 2026-04-10 | 合同会社みやび_

---

## これは何か

LLM エージェントに「全部覚えていてね」と言うのをやめる。
代わりに、**必要な記憶だけをピンポイントで差し込む**仕組みを作る。

その仕組みの上で、**どのマシンのどのエージェントが動いても、同じ手順で同じ結果にたどり着く**状態を作る。

これが MergeGate。全エージェントが従う唯一の指針。

---

## なぜ必要なのか

### LLM は机の広さに限界がある

LLM にはコンテキストウィンドウという「机」がある。
机に書類を山積みにすると、必要な情報が埋もれてミスが増える。

「全部覚えていてね」は机を溢れさせる行為。
実際に起きること:

- 長い会話を流し込むほど、エージェントの精度が落ちる
- 「前に話した」ことを暗黙に期待すると、記憶が消えていて混乱する
- 複数エージェントが同じ情報を別々に記憶すると、食い違いが起きる

### マルチマシンで記憶がバラバラになる

MacBook Pro、Windows、mainmini、macmini2、mini3。
複数のマシンに 39 体のエージェントが動いている。

あるマシンで「タスク A 完了」と記録しても、別のマシンのエージェントはそれを知らない。
MEMORY.md がコピーごとにズレる。
結果、同じ仕事を二度やったり、終わっていないのに終わったと思い込む。

### 「完了」の定義がブレる

エージェントが会話の中で「完了しました」と言っても、GitHub Issue はまだ Open。
逆もある。
「完了とは何か」の基準が、会話テキストという曖昧なものにしかない。

---

## 解決策: 3 つの原則

### 原則 1: 記憶はアタッチメント

エージェントに「全部覚えていてね」と言わない。
代わりに、**そのタスクに必要な情報だけを、短い検証可能な単位で差し込む**。

これを「ピンポイント・アタッチメント」と呼ぶ。

```
❌ やめること:
   「MEMORY.md を全部読んで認証機能を直して」（3000行がコンテキスト窓を圧迫）

✅ やること:
   「Issue #45 の要件で、src/auth/auth.controller.ts の verifyToken() を修正して。
    impact 分析: d=1 で 12 シンボル影響、HIGH リスク」（必要な情報だけ 200 行）
```

アタッチメントの種類:
- **Issue URL** — タスクの定義と完了条件
- **impact 分析結果** — 変更の影響範囲（数値）
- **SKILL.md の該当節** — 手順書の必要な部分だけ
- **ファイル抜粋** — 変更対象のコードだけ

### 原則 2: ジグ環境（手順 + 環境 = 再現性）

「ジグ」とは工場の治具。
ワークを固定して、**誰がやっても同じ加工ができる**ようにする器具。

ソフトウェア開発でのジグ:
- **SKILL.md** — 「この手順でやれ」が書いてあるファイル
- **CLI ツール** — `miyabi gate assign` と打てばロックが取れる
- **GATE** — 条件を満たさなければ次に進めない壁
- **Issue の完了条件** — 「テスト通過 + PR マージ = 完了」と明文化

**十分に閉じた手順と、観測可能なジグが揃えば、その環境内では手順通りに実行可能な状態に近づける。**

これがジグ仮説。
Polaris はこの仮説を実装したもの。

### 原則 3: 二層 SSOT（事実と文脈を分ける）

| 何を管理するか | どこに置くか | なぜ |
|-------------|-----------|-----|
| タスクの状態（誰がやる、終わったか） | **GitHub Issue / Projects** | 全マシンから API で見える。嘘がつけない |
| 手順・仕様・学び（どうやる、なぜそうする） | **リポジトリの docs/** | git commit で版管理。改ざんが追跡できる |

「SSOT」= Single Source of Truth = 唯一の正しい情報源。

GitHub が事実の正。リポが文脈の正。
tasks.json は **実行台帳**（ローカルのロック・DAG・GATE 状態）であり、事実の正ではない。

---

## Polaris が実現する世界

### Before（今の世界）

```
エージェントが「やります」と言う
  → エージェントが「やりました」と言う
    → 人間が「本当に？」と確認しに行く
      → Issue まだ Open。ブランチもない。別のエージェントが同じファイル触ってた。
```

人間が全エージェントの後ろに立って「本当にやった？」と確認する世界。

### After（Polaris の世界）

```
エージェントが「やります」と言う
  → tasks.json: Issue がない → 登録拒否。Issue 作れ。
  → tasks.json: 依存が done じゃない → blocked。待て。
  → tasks.json: impact が null → analyzing のまま。GNI 回せ。
  → tasks.json: ファイルロック競合 → ロック拒否。先のタスクが終わるまで待て。
  → tasks.json: ロック獲得 → implementing。やっと作業開始。
  → tasks.json: PR がない → reviewing に遷移不可。PR 作れ。
  → tasks.json: merge commit がない → done に遷移不可。マージしろ。
  → tasks.json: merge commit = "a1b2c3..." → done。ロック解放。後続タスク解放。
```

**エージェントの発言は一切関与しない。JSON のフィールドだけが門番。**

tasks.json が自動的に検証し、人間は CRITICAL リスクの承認だけすればいい世界。

---

## 仕組み: GATE チェーン

9 つの GATE が、タスクの draft → done を一本道で制御する。
どの GATE も飛ばせない。LLM が「大丈夫です」と言っても、GATE が閉じていれば進めない。

```
GATE 0: Issue が存在するか
GATE 1: タイトルと説明があるか
GATE 2: 依存タスクが全て完了しているか
GATE 3: GNI impact 分析が記録されているか（HIGH なら人間承認）
GATE 4: ファイルロックが取れるか（競合なら拒否）
GATE 5: ブランチ名が正しい形式か
GATE 6: PR が作成されているか
GATE 7: merge commit が GitHub API で検証されたか
GATE 8: Issue が Close されて監査ログに記録されたか
```

全 GATE は **JSON フィールドの値チェック**。
LLM の自然言語判断は一切介在しない。

---

## 仕組み: ファイルロック

2 つのエージェントが同じファイルを同時に編集することを防ぐ。

```
Agent A が src/auth.rs をロック
  → tasks.json の file_locks に "src/auth.rs": "task-001" と記録
  → Agent B が同じファイルに触ろうとする
  → tasks.json を読む → ロック済み → 拒否
  → Agent A の作業が終わる → ロック解放
  → Agent B が再度試みる → ロック取得成功
```

lease + heartbeat 方式:
- ロックは 300 秒の lease（借用期間）
- 60 秒ごとに heartbeat（生存確認）
- heartbeat が 2 回途切れたら stale（放棄）と判定してロック自動解放

---

## 仕組み: DAG（依存関係）

タスク間の「先にこれをやらないと次に進めない」関係を DAG（有向非循環グラフ）で定義。

```
Level 0: DB スキーマ変更 (task-000)
          ↓ 完了するまで次に進めない
Level 1: API 実装 (task-001)
          ↓ 完了するまで次に進めない
Level 2: フロントエンド (task-002) + テスト (task-003) ← 並列実行可能
```

DAG のレベル計算は Kahn のトポロジカルソート（miyabi-core の dag.rs に 823 行で実装済み）。

---

## 仕組み: 記憶のライフサイクル

```
当日のメモ (memory/2026-04-10.md)
  ↓ 1日の終わりに振り返り
  ↓
残す？ → Yes → どこに昇格？
                ├── タスク状態 → GitHub Issue に書く
                └── 学び・仕様 → docs/ にコミット
残す？ → No  → 捨てる（アーカイブ）
```

当日のメモは「SSOT にしない」。
昇格されて初めて正式な記録になる。
エージェントが当日メモだけを根拠に「完了した」と判断してはいけない。

---

## 技術スタック

| 要素 | 技術 | 行数 | 状態 |
|------|------|------|------|
| DAG エンジン | Rust (dag.rs) | 823 | ✅ 実装済み |
| GitHub 連携 | Rust (github.rs) | 947 | ✅ 実装済み |
| 承認ゲート | Rust (approval.rs) | 231 | ✅ 実装済み |
| 並列実行 | Rust (orchestration.rs) | 451 | ✅ 実装済み |
| ワークフロー | Rust (workflow.rs) | 889 | ✅ 実装済み |
| セッション管理 | Rust (session.rs) | 466 | ✅ 実装済み |
| OpenClaw 連携 | Rust (openclaw.rs) | 354 | ✅ 実装済み |
| GATE 検証 | Rust (gate.rs) | ~100 | ✅ Phase A で追加 |
| ファイルロック | Rust (lock.rs) | ~100 | ✅ Phase A で追加 |
| プロトコル統合 | Rust (protocol.rs) | ~150 | ✅ Phase A で追加 |
| tasks.json 永続化 | Rust (store.rs) | ~50 | ✅ Phase A で追加 |
| CLI | Rust (main.rs) | ~365 | ✅ Phase B で追加 |

全て Rust。`unsafe_code = "forbid"`。clippy 全警告をエラー化。
**コンパイルが通る = 型安全 + メモリ安全 + lint 通過が保証される。**

---

## CLI インターフェース

```bash
miyabi gate register --issue 45 --title "認証移行"     # タスク登録
miyabi gate status                                      # 全タスク状態表示
miyabi gate dispatchable                                # 実行可能タスク一覧
miyabi gate impact task-001 --risk HIGH --symbols 12    # impact 記録
miyabi gate assign task-001 --agent codex --node mac    # ロック獲得 + 実装開始
miyabi gate branch task-001 feature/issue-45-auth       # ブランチ記録
miyabi gate pr task-001 78                              # PR 記録
miyabi gate merge task-001                              # merge 検証 + 完了
miyabi gate locks                                       # ロック一覧
miyabi gate dag                                         # DAG 可視化
```

エージェントはこの CLI を呼ぶだけ。
CLI が GATE を検証し、tasks.json を更新し、exit code で結果を返す。
exit 0 = 成功。exit 1 = GATE 拒否。exit 2 = 入力エラー。

---

## OpenClaw との統合

OpenClaw main エージェントが Polaris CLI を呼ぶ:

```
OpenClaw main
  → miyabi gate register --issue 45    （タスク登録）
  → miyabi gate dispatchable           （実行可能タスク取得）
  → sessions spawn --agent kade        （カエデを起動）
  → カエデ: miyabi gate assign task-001（ロック獲得）
  → カエデ: 実装
  → カエデ: miyabi gate pr task-001 78 （PR 記録）
  → sessions spawn --agent sakura      （サクラにレビュー依頼）
  → サクラ: レビュー → Approve
  → miyabi gate merge task-001         （merge 検証 + 完了）
  → 後続タスクが自動解放
```

---

## ジグ仮説の検証

> **十分に閉じた手順と、観測可能なジグが揃えば、その環境内では手順通りに実行可能な状態に近づける。**

Polaris がジグとして提供するもの:
- **GATE** — 条件を満たさなければ進めない壁
- **tasks.json** — 全タスクの状態を JSON で一元管理
- **CLI** — 全操作をコマンドで実行（曖昧な指示が不可能）
- **DAG** — 依存関係を強制（飛ばせない）
- **ファイルロック** — 競合を防止（同時編集が不可能）
- **GitHub SSOT** — 完了の定義が明確（merge されたかどうか）

これだけの手順を与えたら、**外部環境をジグとしてアタッチメントすると、その環境の中ではその手順通りに全てできるような状態になるはず**。

人格と記憶は別。能力と記憶は別。
**ノウハウを与えられたら誰でもできるはず。**
そこに持ち込むのが Polaris の目的。

---

## 成立する条件

以下を満たすとき、**実行ノードを変えても同等の手順でタスクを進められる**:

1. **GitHub Issue**（ファクト SSOT）がネットワーク経由で同じ
2. **対象リポジトリ**がそのノードに checkout またはマウントされている
3. **GNI** がそのリポに対して分析済み
4. **miyabi gate CLI** がインストールされている

このとき「プロトコルと SSOT が同じ」なので、**エージェントの人格はノードごとに違っても、判断の根拠は揃う**。

## 成立しない条件

以下は **誤解**:

- 同一セッションがどのマシンにも瞬時に存在する → **偽**。セッションはノードローカル。
- 全マシンに同じツールがインストールされている → **偽**。Smart Connections は MacBook だけ。
- tasks.json を共有すれば OpenClaw のメモリも一致する → **偽**。ランタイム索引は別ストア。

---

## まとめ

```
LLM の出力（テキスト）  ← 信用しない
         ↓ 検証
tasks.json の GATE     ← ステートマシンが許可/拒否を決定
         ↓ 同期
GitHub Issue/PR の状態 ← リモート SSOT として全マシンに公開
         ↓ 不可逆
git merge              ← 確定。ここまで来て初めて「完了」
```

**LLM は提案する。tasks.json が許可する。GitHub が確定する。**

この三段階で揺らぎを殺す。
これが Polaris。北極星。

---

_Created by 林 駿甫 (Shunsuke Hayashi) with Claude Opus 4.6_
_ShunsukeHayashi / miyabi-cli-standalone_
