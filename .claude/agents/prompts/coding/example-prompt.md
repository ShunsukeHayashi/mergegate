# Example Agent Prompt

このファイルは、Agentプロンプトのサンプルです。

## プロンプト構造

```markdown
# [Task名]

## Context
プロジェクトのコンテキスト情報

## Objective
このAgentが達成すべき目標

## Inputs
- Issue URL: https://github.com/user/repo/issues/123
- Task ID: TASK-456
- Dependencies: TASK-123, TASK-124

## Instructions
1. ステップ1を実行
2. ステップ2を実行
3. ...

## Output Format
期待される出力形式（JSON, Markdown, Code等）

## Success Criteria
- 基準1
- 基準2
```

## 使用方法

Worktree内でClaude Codeセッションを起動する際、このプロンプトが自動的に読み込まれます。

```bash
cd .worktrees/issue-123
# Claude Codeがこのプロンプトを参照して実行
```

## カスタマイズ

プロジェクト固有のプロンプトを作成する場合：

1. このファイルをコピー
2. 内容をカスタマイズ
3. `.claude/agents/prompts/coding/` に配置

## 🔗 参考

実際のプロンプト例は、Miyabiプロジェクトの `.claude/agents/prompts/coding/` を参照してください。
