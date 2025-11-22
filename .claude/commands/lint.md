Lintを実行してコード品質をチェックしてください。

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

警告やエラーがあれば修正してください。
