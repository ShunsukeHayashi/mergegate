# Troubleshooting Guide

Miyabi使用中に発生する可能性のある問題と解決策をまとめています。

## 🔧 環境関連

### GITHUB_TOKENが設定されていない

**症状**:
```
Error: GITHUB_TOKEN not set
```

**解決策**:
1. トークンを取得: https://github.com/settings/tokens/new
2. 環境変数を設定:
   ```bash
   export GITHUB_TOKEN=ghp_xxxxx
   ```
3. シェルプロファイルに追加（永続化）:
   ```bash
   echo 'export GITHUB_TOKEN=ghp_xxxxx' >> ~/.zshrc
   source ~/.zshrc
   ```

### ANTHROPIC_API_KEYが設定されていない

**症状**:
```
Error: ANTHROPIC_API_KEY not set
```

**解決策**:
1. Anthropic Consoleでキー取得: https://console.anthropic.com/
2. 環境変数を設定:
   ```bash
   export ANTHROPIC_API_KEY=sk-ant-xxxxx
   ```

## 🐛 Git関連

### Git repositoryが見つからない

**症状**:
```
Error: Not a git repository
```

**解決策**:
```bash
cd your-project
git init
```

### Worktreeが残ったまま

**症状**:
```
Error: Worktree already exists: .worktrees/issue-123
```

**解決策**:
```bash
# Worktree一覧確認
git worktree list

# 不要なWorktreeを削除
git worktree remove .worktrees/issue-123

# すべてのstale Worktreeをクリーンアップ
git worktree prune
```

### マージコンフリクト

**症状**:
```
CONFLICT (content): Merge conflict in src/main.rs
```

**解決策**:
```bash
# コンフリクトファイルを確認
git status

# 手動でコンフリクトを解決
# エディタでファイルを開き、<<<<<<<, =======, >>>>>>> マーカーを削除

# 解決後
git add src/main.rs
git commit -m "fix: resolve merge conflict"
```

## 🤖 Agent関連

### Agent実行が失敗する

**症状**:
```
Error: Agent execution failed
```

**解決策**:
1. ログファイル確認:
   ```bash
   tail -f logs/miyabi-$(date +%Y%m%d).log
   ```
2. Issue番号が正しいか確認:
   ```bash
   gh issue list
   ```
3. Labelが正しく設定されているか確認

### Issue番号が見つからない

**症状**:
```
Error: Issue #123 not found
```

**解決策**:
```bash
# Issue一覧確認
gh issue list --limit 50

# 正しい番号で再実行
miyabi agent run coordinator --issue 正しい番号
```

## 📦 インストール関連

### `miyabi` コマンドが見つからない

**症状**:
```
command not found: miyabi
```

**解決策**:
```bash
# crates.ioから再インストール
cargo install miyabi-cli --force

# パス確認
which miyabi

# Cargo binディレクトリがPATHに含まれているか確認
echo $PATH | grep -o "$HOME/.cargo/bin"

# なければ追加
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### コンパイルエラー

**症状**:
```
error: failed to compile miyabi-cli
```

**解決策**:
```bash
# Rustを最新版にアップデート
rustup update stable

# Cargoキャッシュをクリア
rm -rf ~/.cargo/registry
rm -rf ~/.cargo/git

# 再インストール
cargo install miyabi-cli
```

## 🌐 GitHub関連

### API Rate Limit

**症状**:
```
Error: API rate limit exceeded
```

**解決策**:
1. GitHubにログイン状態か確認:
   ```bash
   gh auth status
   ```
2. Personal Access Tokenのスコープ確認
3. しばらく待つ（Rate limitは1時間でリセット）

### Permission denied

**症状**:
```
Error: Resource not accessible by personal access token
```

**解決策**:
1. トークンのスコープを確認: `repo`, `workflow` が必要
2. 新しいトークンを生成
3. 環境変数を更新

## 🔍 デバッグ方法

### 詳細ログを有効化

`.miyabi.yml` でログレベルを変更:
```yaml
logging:
  level: debug  # info → debug
  directory: "./logs"
```

### ログファイルの確認

```bash
# 最新のログ
tail -100 logs/miyabi-$(date +%Y%m%d).log

# エラーのみ抽出
grep -i "error\|fail" logs/miyabi-*.log

# 特定のAgentのログ
grep -i "CoordinatorAgent" logs/miyabi-*.log
```

### miyabi status の活用

```bash
miyabi status

# 出力例:
# Miyabi Installation: ✅ or ❌
# Environment: GITHUB_TOKEN, ANTHROPIC_API_KEY の状態
# Git Repository: ブランチ、コミット状態
# Worktrees: アクティブなWorktree数
```

## 🆘 それでも解決しない場合

### サポートを受ける

1. **GitHub Issues**: https://github.com/ShunsukeHayashi/Miyabi/issues
   - 詳細なエラーメッセージを含めてください
   - `miyabi status` の出力を添付
   - ログファイルの関連部分を添付

2. **Discord Community**: （準備中）

3. **ドキュメント**:
   - [GETTING_STARTED.md](GETTING_STARTED.md)
   - [CLAUDE.md](../CLAUDE.md)
   - [GitHub Discussions](https://github.com/ShunsukeHayashi/Miyabi/discussions)

### 報告に含めるべき情報

```bash
# システム情報
uname -a

# Rustバージョン
rustc --version
cargo --version

# miyabiバージョン
miyabi --version

# プロジェクト状態
miyabi status

# 直近のログ
tail -50 logs/miyabi-$(date +%Y%m%d).log
```

---

**Miyabi** - Beauty in Autonomous Development 🌸
