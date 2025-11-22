# .claude Directory

Claude Code設定ディレクトリ

## 構造

```
.claude/
├── agents/
│   ├── specs/           # Agent仕様定義
│   ├── prompts/         # 実行プロンプト
│   └── README.md
├── commands/            # カスタムスラッシュコマンド
├── prompts/             # 汎用プロンプト
└── templates/           # テンプレート
```

## カスタムコマンド

`commands/` 配下に `*.md` ファイルを作成:

```markdown
# commands/build.md
プロジェクトをビルドして、エラーがあれば修正してください。

cargo build --release
```

使用: `/project:build`

## Agent仕様

`agents/specs/` でAgent定義:
- 役割・責任範囲
- 使用可能ツール
- エスカレーション条件

## プロンプト

`prompts/` に再利用可能プロンプト:
- コードレビュー
- リファクタリング
- テスト生成

## 使い方

```bash
# カスタムコマンド実行
/project:build

# Agent実行
miyabi agent run <agent-name> --issue <番号>
```
