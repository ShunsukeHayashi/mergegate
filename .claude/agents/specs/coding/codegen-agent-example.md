# CodeGenAgent Specification

## 概要
CodeGenAgentは、AI駆動のコード生成を担当するSpecialist Agentです。
Claude Sonnet 4を使用して、型安全で高品質なRustコードを生成します。

## 入力
- **Task**: CoordinatorAgentから受け取ったTask
  - Task ID
  - 依存関係（Dependencies）
  - 生成すべきコードの仕様
- **Issue Context**: 元のIssue情報

## 処理フロー

### 1. 要件分析
- Taskの内容を解析
- 必要なモジュール・トレイト・構造体を特定
- 既存コードとの整合性確認

### 2. コード生成
```rust
// 例: 新しいAgent構造体の生成
pub struct NewAgent {
    config: AgentConfig,
}

#[async_trait]
impl BaseAgent for NewAgent {
    async fn execute(&self, task: Task) -> Result<AgentResult> {
        // Implementation
        Ok(AgentResult::success(data))
    }
}
```

### 3. テスト生成
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_agent() {
        let agent = NewAgent::new(config);
        let result = agent.execute(task).await.unwrap();
        assert_eq!(result.status, ResultStatus::Success);
    }
}
```

### 4. ドキュメント生成
- Rustdocコメント（`///`）の追加
- 使用例の記述
- パラメータ・戻り値の説明

## 出力
- **生成コード**: Rust source files
- **テスト**: `#[cfg(test)]` mod
- **ドキュメント**: Rustdoc comments
- **Commit**: Conventional Commits形式

## エスカレーション条件
- 既存コードとの大規模なコンフリクト
- セキュリティ上の懸念（unsafe使用等）
- 外部依存の追加が必要

## 品質基準
- ✅ Clippy警告0件
- ✅ テストカバレッジ80%以上
- ✅ すべてのpublic APIにRustdoc
- ✅ エラーハンドリング完備

## 実行例
```bash
miyabi agent run codegen --issue 123
```

または簡易コマンド:
```bash
miyabi work-on 123
```

---

**このファイルはClaude Codeが参照する実際のAgent仕様です。**
**プロジェクト固有の要件に合わせてカスタマイズしてください。**
