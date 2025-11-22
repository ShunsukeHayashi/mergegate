# Coding Agent 仕様

このディレクトリには、コーディング系Agent（7種類）の仕様を配置します。

## Agent仕様ファイルのフォーマット

各Agentの仕様は以下の構造で記述します：

```markdown
# [Agent名] 仕様

## 概要
Agentの役割と責任範囲

## 入力
- 受け取るTask/Issueの形式
- 必須パラメータ

## 処理フロー
1. ステップ1
2. ステップ2
3. ...

## 出力
- 生成する成果物
- 更新するIssue/PR

## エスカレーション条件
- 上位Agentへのエスカレーション基準
- エラーハンドリング

## 実行例
\`\`\`bash
miyabi agent run [agent-type] --issue 123
\`\`\`
```

## 📋 テンプレート

新しいAgent仕様を作成する場合、以下のテンプレートを使用してください：

```bash
cp example-agent-spec.md my-custom-agent.md
```

## 🔗 参考

既存のAgent仕様は以下を参照：
- Miyabiプロジェクトの `.claude/agents/specs/coding/` ディレクトリ
- [Agent Operations Manual](https://github.com/ShunsukeHayashi/Miyabi/blob/main/docs/AGENT_OPERATIONS_MANUAL.md)
