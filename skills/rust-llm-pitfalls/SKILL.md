# Rust LLM Pitfalls — LLM が Rust を書くとき間違いやすいポイント

> LLM（Claude, GPT, Codex）が Rust コードを生成する際に頻出するミス。
> エージェントはこのリストを実装前に必ず読み、該当パターンを避けること。

## 1. 所有権・借用

### 1.1 ❌ clone() の乱用

LLM は borrow checker エラーを `.clone()` で解決しようとする。

```rust
// ❌ LLM がやりがち
let tasks = self.store.tasks().clone();  // Vec 全体をコピー
for task in &tasks { ... }

// ✅ 正しい: 参照で十分
for task in self.store.tasks() { ... }
```

**ルール**: `clone()` を書く前に「参照で済まないか」を考える。`clone()` は最後の手段。

### 1.2 ❌ 可変参照と不変参照の同時使用

```rust
// ❌ LLM がやりがち: 同じ構造体から可変と不変を同時に借用
let task = self.store.get_task(id);      // &self（不変参照）
self.store.update_task(id, |t| { ... }); // &mut self（可変参照）← コンパイルエラー

// ✅ 正しい: スコープを分ける
let state = {
    let task = self.store.get_task(id).unwrap();
    task.state  // Copy 型なのでスコープ外に持ち出せる
};
self.store.update_task(id, |t| { t.state = state; });
```

### 1.3 ❌ String と &str の混同

```rust
// ❌ LLM がやりがち
fn find_task(id: String) -> Option<&Task> { ... }

// ✅ 正しい: 入力は &str、保存は String
fn find_task(id: &str) -> Option<&Task> { ... }
```

## 2. エラー処理

### 2.1 ❌ unwrap() の乱用

LLM は `Option` / `Result` を `unwrap()` で解決しようとする。

```rust
// ❌ 本番コードで unwrap → panic
let task = store.get_task(id).unwrap();

// ✅ 正しい: ? 演算子 or 明示的エラー
let task = store.get_task(id)
    .ok_or_else(|| StoreError::UnknownTask(id.to_string()))?;
```

**ルール**: `unwrap()` はテストコード内のみ許可。本番コードでは `?` を使う。

### 2.2 ❌ エラー型の設計不足

LLM は `String` をエラー型にしようとする。

```rust
// ❌ LLM がやりがち
fn register_task(task: Task) -> Result<(), String> { ... }

// ✅ 正しい: 専用 enum
fn register_task(task: Task) -> Result<(), ProtocolError> { ... }
```

## 3. serde / JSON

### 3.1 ❌ rename_all の不整合

```rust
// ❌ snake_case と SCREAMING_SNAKE_CASE が混在
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    LOW,      // → "low" になる（意図と違う？）
    MEDIUM,
}

// ✅ 正しい: 意図を明確にする
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]  // JSON では "LOW", "MEDIUM"
// または
#[serde(rename_all = "snake_case")]  // JSON では "low", "medium"
```

### 3.2 ❌ Option フィールドのスキップ忘れ

```rust
// ❌ null が JSON に出力される
pub merge_commit: Option<String>,  // → "merge_commit": null

// ✅ null を消したいなら
#[serde(skip_serializing_if = "Option::is_none")]
pub merge_commit: Option<String>,  // → フィールドごと消える
```

### 3.3 ❌ DateTime のシリアライズ形式

```rust
// ❌ LLM が独自フォーマットを書こうとする
pub created_at: String,  // "2026-04-10 00:00:00" ← 危険

// ✅ 正しい: chrono::DateTime<Utc> + serde
pub created_at: DateTime<Utc>,  // ISO 8601 自動
```

## 4. 並行性・ファイル操作

### 4.1 ❌ ファイル操作の非原子性

```rust
// ❌ LLM がやりがち: read → modify → write が原子的でない
let content = fs::read_to_string("tasks.json")?;
let mut doc: TasksDocument = serde_json::from_str(&content)?;
doc.tasks.push(new_task);
fs::write("tasks.json", serde_json::to_string_pretty(&doc)?)?;
// ↑ 書き込み中にクラッシュすると tasks.json が壊れる

// ✅ 正しい: atomic rename
let tmp = "tasks.json.tmp";
fs::write(tmp, serde_json::to_string_pretty(&doc)?)?;
fs::rename(tmp, "tasks.json")?;  // OS レベルで原子的
```

### 4.2 ❌ flock の使い方

```rust
// ❌ LLM が flock を忘れる
let content = fs::read_to_string(path)?;
// ↑ 別プロセスが同時に読んでいる可能性

// ✅ 正しい: fs2 で排他ロック
use fs2::FileExt;
let file = File::open(path)?;
file.lock_exclusive()?;  // 他プロセスをブロック
let content = fs::read_to_string(path)?;
// ... 操作 ...
file.unlock()?;
```

### 4.3 ❌ HashMap の順序依存

```rust
// ❌ LLM が HashMap の順序に依存するコードを書く
let locks: HashMap<String, String> = ...;
for (file, task_id) in &locks {
    // 順序は保証されない！テストが環境で変わる
}

// ✅ 正しい: BTreeMap を使うか、ソートしてから処理
let mut files: Vec<_> = locks.keys().collect();
files.sort();
```

## 5. テスト

### 5.1 ❌ テストで実ファイルシステムを汚す

```rust
// ❌ /tmp に直接書く → テスト並列実行で衝突
fs::write("/tmp/test-tasks.json", "...")?;

// ✅ 正しい: tempfile crate
use tempfile::TempDir;
let dir = TempDir::new()?;
let path = dir.path().join("tasks.json");
```

### 5.2 ❌ 時刻依存テスト

```rust
// ❌ Utc::now() に依存 → テストが時間で変わる
let lock = TaskLock { locked_at: Utc::now(), ttl_secs: 300, ... };
assert!(!lock.is_expired(Utc::now()));  // 瞬間的に通るが不安定

// ✅ 正しい: 固定時刻を注入
let now = Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap();
let lock = TaskLock { locked_at: now, ttl_secs: 300, ... };
assert!(!lock.is_expired(now + chrono::Duration::seconds(100)));
assert!(lock.is_expired(now + chrono::Duration::seconds(500)));
```

## 6. clippy 頻出警告

LLM が生成するコードで clippy が止めるもの:

| 警告 | 原因 | 対処 |
|------|------|------|
| `needless_pass_by_value` | 引数を値で受けて消費しない | `&` に変更 |
| `derivable_impls` | 手書き Default が derive で済む | `#[derive(Default)]` |
| `cast_possible_wrap` | `u64 as i64` のオーバーフロー | `.try_into()` or `.cast_signed()` |
| `unnested_or_patterns` | matches! の書き方が冗長 | パターンをネスト |
| `redundant_clone` | 不要な clone | 削除 |
| `single_match` | match で1パターンだけ | if let に変更 |

## このスキルの使い方

実装前にこのリストを一読し、該当パターンを避ける。
clippy が deny なので、上記の多くはコンパイル時に自動検出される。
ただし **ロジックの間違い（unwrap の本番使用、ファイル操作の非原子性）は clippy では検出されない**。人間またはレビューエージェントが確認する。
