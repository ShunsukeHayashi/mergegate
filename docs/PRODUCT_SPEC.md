# Product Specification - MergeGate

**Version**: 1.0.0
**Status**: Draft
**Last Updated**: 2026-04-12

---

## 1. Executive Summary

### 1.1 Product Vision
AIコーディングエージェントの前段に置く、engine-agnostic な gate CLI を提供する。
現在の製品中心は TUI ではなく `mergegate gate ...` であり、ledger の記録・検証・可視化を最優先に進める。
本流方針として、MergeGate は Rust-first の deterministic execution protocol product として進め、cross-source PM dashboard 化は本体スコープに含めない。

### 1.2 Target Users
- ソフトウェア開発者
- DevOpsエンジニア
- AI/MLエンジニア

### 1.3 Core Value Proposition
- **高速**: Rust実装による100ms以下の起動時間
- **安全**: lock / impact / merge evidence を明示的に記録
- **監査可能**: validate / export / stats / dashboard で ledger を点検できる
- **拡張性**: OpenClaw や外部自動化へ JSON 出力で接続できる

### 1.4 Current Product Surface

現在の安定インターフェースは以下:

- `mergegate gate status`
- `mergegate gate validate`
- `mergegate gate export-json`
- `mergegate gate export-md`
- `mergegate gate stats`
- `mergegate gate serve`

直近の固定契約:

- `mergegate gate validate` は text 先頭に `clean` / `warning` / `error` を出し、JSON では severity / exit_code / issue_count を返す
- `mergegate gate export-json` と `mergegate gate export-md` は共通 filter `--state` / `--risk` / `--since` を使う
- dashboard は `/api/tasks` `/api/stats` `/api/validate` を使い、CLI と同じ ledger 定義を表示する

TUI / built-in chat runtime / vendor-specific execution機能は将来拡張であり、現段階では必須スコープではない。
また、portfolio / cross-source orchestration / PM cockpit は上位レイヤーとして扱い、MergeGate 本体の定義には含めない。

---

## 2. Functional Requirements

> Note: 以下の TUI/LLM 要件は将来ロードマップとして保持している。直近開発の primary surface は gate CLI と web dashboard である。

### 2.1 TUI System

#### FR-TUI-001: Markdown Streaming Renderer

**Description**: LLMレスポンスをリアルタイムでレンダリング

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TUI-001-1 | Character-by-character streaming | Must | Pending |
| FR-TUI-001-2 | Progressive code block rendering | Must | Pending |
| FR-TUI-001-3 | Syntax highlighting (50+ languages) | Must | Pending |
| FR-TUI-001-4 | Cursor position tracking | Should | Pending |
| FR-TUI-001-5 | Auto-scroll during streaming | Should | Pending |
| FR-TUI-001-6 | User scroll override | Could | Pending |

**Acceptance Criteria**:
```gherkin
Scenario: Streaming text display
  Given the LLM is generating a response
  When text chunks arrive
  Then each character is displayed immediately
  And the display scrolls to show latest content
  And there is no visible flickering
```

**Performance**:
- Render latency: < 16ms (60 FPS)
- Memory: < 10MB for 100K characters
- CPU: < 5% during streaming

---

#### FR-TUI-002: Git Diff Visualization

**Description**: Git diffの美しい視覚化

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TUI-002-1 | Unified diff parsing | Must | Pending |
| FR-TUI-002-2 | Color-coded additions/deletions | Must | Pending |
| FR-TUI-002-3 | Line numbers (old/new) | Must | Pending |
| FR-TUI-002-4 | Syntax highlighting in diff | Should | Pending |
| FR-TUI-002-5 | Hunk navigation (n/N) | Should | Pending |
| FR-TUI-002-6 | Split view mode | Could | Pending |

**Color Scheme**:
```
Addition:  #2ea043 (Green)
Deletion:  #f85149 (Red)
Context:   Default
Hunk header: #8b949e (Gray)
```

**Acceptance Criteria**:
```gherkin
Scenario: View file diff
  Given a unified diff output
  When rendered in the TUI
  Then additions are highlighted in green
  And deletions are highlighted in red
  And line numbers are correctly aligned
```

---

#### FR-TUI-003: Chat Composer

**Description**: マルチライン入力とコマンド補完

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TUI-003-1 | Multi-line text input | Must | Pending |
| FR-TUI-003-2 | Command history (Up/Down) | Must | Pending |
| FR-TUI-003-3 | Tab completion | Should | Pending |
| FR-TUI-003-4 | Syntax highlighting in input | Should | Pending |
| FR-TUI-003-5 | Vim keybindings | Could | Pending |
| FR-TUI-003-6 | Emacs keybindings | Could | Pending |

**Key Bindings**:
```
Enter       : Send message (single line)
Shift+Enter : New line
Ctrl+C      : Cancel
Ctrl+L      : Clear screen
Up/Down     : History navigation
Tab         : Auto-complete
```

---

#### FR-TUI-004: Text Area Widget

**Description**: 完全機能テキストエディタ

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TUI-004-1 | Text selection (Shift+Arrow) | Must | Pending |
| FR-TUI-004-2 | Copy/Paste (Ctrl+C/V) | Must | Pending |
| FR-TUI-004-3 | Undo/Redo (Ctrl+Z/Y) | Must | Pending |
| FR-TUI-004-4 | Word wrap | Should | Pending |
| FR-TUI-004-5 | Find & Replace | Could | Pending |

---

#### FR-TUI-005: Approval Overlay

**Description**: ツール実行の承認UI

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TUI-005-1 | Show tool name and arguments | Must | Pending |
| FR-TUI-005-2 | Y/N/A/N options | Must | Pending |
| FR-TUI-005-3 | Risk level indicator | Should | Pending |
| FR-TUI-005-4 | Remember "Always" choices | Should | Pending |
| FR-TUI-005-5 | Preview changes | Could | Pending |

**Risk Levels**:
- 🟢 Low: Read operations
- 🟡 Medium: Write to files
- 🔴 High: Execute commands, delete files

---

### 2.2 LLM Integration

#### FR-LLM-001: Anthropic API Client

**Description**: Claude APIとの通信

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-LLM-001-1 | POST /v1/messages | Must | Pending |
| FR-LLM-001-2 | SSE streaming | Must | Pending |
| FR-LLM-001-3 | API key authentication | Must | Pending |
| FR-LLM-001-4 | Error handling (4xx, 5xx) | Must | Pending |
| FR-LLM-001-5 | Retry with exponential backoff | Must | Pending |
| FR-LLM-001-6 | Request timeout (30s default) | Should | Pending |

**API Endpoint**:
```
POST https://api.anthropic.com/v1/messages
Headers:
  x-api-key: $ANTHROPIC_API_KEY
  anthropic-version: 2023-06-01
  content-type: application/json
```

**Error Handling**:
| Code | Action |
|------|--------|
| 400 | Show error, don't retry |
| 401 | Prompt for API key |
| 429 | Wait, retry with backoff |
| 500+ | Retry up to 3 times |

---

#### FR-LLM-002: Conversation Management

**Description**: 会話履歴と状態管理

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-LLM-002-1 | Store message history | Must | Pending |
| FR-LLM-002-2 | System prompt support | Must | Pending |
| FR-LLM-002-3 | Conversation persistence | Should | Pending |
| FR-LLM-002-4 | Resume from previous session | Should | Pending |
| FR-LLM-002-5 | Export conversation | Could | Pending |

**Data Model**:
```rust
struct Message {
    role: Role,        // user, assistant, system
    content: Content,  // text, tool_use, tool_result
    timestamp: DateTime<Utc>,
}

struct Conversation {
    id: Uuid,
    messages: Vec<Message>,
    system_prompt: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

**Storage**:
- Location: `~/.miyabi/conversations/`
- Format: JSON
- Retention: 30 days default

---

#### FR-LLM-003: Token Management

**Description**: トークンカウントとコンテキスト管理

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-LLM-003-1 | Token estimation | Must | Pending |
| FR-LLM-003-2 | Display current usage | Must | Pending |
| FR-LLM-003-3 | Auto-prune old messages | Must | Pending |
| FR-LLM-003-4 | Warning at 80% capacity | Should | Pending |
| FR-LLM-003-5 | Manual context clear | Should | Pending |

**Limits**:
| Model | Context Window | Reserve |
|-------|----------------|---------|
| Claude Sonnet | 200K | 10K |
| Claude Haiku | 200K | 5K |

**Pruning Strategy**:
1. Keep system prompt always
2. Keep last N messages
3. Summarize old messages

---

#### FR-LLM-004: Tool Calling

**Description**: Function calling の実装

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-LLM-004-1 | Send tool definitions | Must | Pending |
| FR-LLM-004-2 | Parse tool_use blocks | Must | Pending |
| FR-LLM-004-3 | Execute tool | Must | Pending |
| FR-LLM-004-4 | Send tool_result | Must | Pending |
| FR-LLM-004-5 | Multi-turn tool use | Must | Pending |

**Tool Schema Format**:
```json
{
  "name": "read_file",
  "description": "Read the contents of a file",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Absolute path to the file"
      }
    },
    "required": ["path"]
  }
}
```

---

### 2.3 Tool System

#### FR-TOOL-001: Tool Registry

**Description**: ツールの登録と管理

**Requirements**:
| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-TOOL-001-1 | Register tool by name | Must | Pending |
| FR-TOOL-001-2 | Lookup tool | Must | Pending |
| FR-TOOL-001-3 | List all tools | Should | Pending |
| FR-TOOL-001-4 | Dynamic registration | Could | Pending |

**Tool Trait**:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, input: Value) -> Result<ToolResult>;
}
```

---

#### FR-TOOL-002: Read Tool

**Description**: ファイル読み込み

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Absolute path |
| offset | number | No | Start line (0-based) |
| limit | number | No | Max lines (default: 2000) |

**Output**:
```
Line numbered content (cat -n format)
```

**Errors**:
- File not found
- Permission denied
- Binary file

---

#### FR-TOOL-003: Write Tool

**Description**: ファイル書き込み

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Absolute path |
| content | string | Yes | Content to write |

**Behavior**:
- Create directories if needed
- Overwrite existing file
- Preserve permissions

---

#### FR-TOOL-004: Edit Tool

**Description**: ファイル編集

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Absolute path |
| old_string | string | Yes | Text to find |
| new_string | string | Yes | Replacement |
| replace_all | bool | No | Replace all occurrences |

**Behavior**:
- Exact string match
- Fail if not unique (unless replace_all)
- Preserve indentation

---

#### FR-TOOL-005: Bash Tool

**Description**: コマンド実行

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| command | string | Yes | Command to execute |
| timeout | number | No | Timeout in ms (default: 120000) |
| cwd | string | No | Working directory |

**Security**:
- Timeout enforcement
- Output truncation (30K chars)
- Dangerous command warning

---

#### FR-TOOL-006: Glob Tool

**Description**: ファイルパターン検索

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| pattern | string | Yes | Glob pattern (e.g., "**/*.rs") |
| path | string | No | Base directory |

**Output**:
- Matching file paths
- Sorted by modification time

---

#### FR-TOOL-007: Grep Tool

**Description**: テキスト検索

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| pattern | string | Yes | Regex pattern |
| path | string | No | Search path |
| type | string | No | File type (e.g., "rs") |
| output_mode | string | No | content/files/count |

**Output Modes**:
- `content`: Matching lines with context
- `files_with_matches`: File paths only
- `count`: Match counts

---

## 3. Non-Functional Requirements

### 3.1 Performance

| Metric | Requirement |
|--------|-------------|
| Startup time | < 100ms |
| Memory usage | < 50MB idle, < 200MB active |
| Response latency | < 16ms (60 FPS) |
| File operation | < 100ms for 1MB file |

### 3.2 Reliability

| Metric | Requirement |
|--------|-------------|
| Crash rate | < 0.1% |
| Error recovery | Graceful degradation |
| Data integrity | No data loss on crash |

### 3.3 Security

| Area | Requirement |
|------|-------------|
| API keys | Environment variables only |
| File access | Validate paths, prevent traversal |
| Command execution | Timeout, sandboxing |
| Permissions | Follow least privilege |

### 3.4 Usability

| Aspect | Requirement |
|--------|-------------|
| Learning curve | < 5 minutes for basic use |
| Error messages | Clear, actionable |
| Help system | Built-in --help |
| Keyboard shortcuts | Discoverable, consistent |

### 3.5 Compatibility

| Platform | Support |
|----------|---------|
| macOS | 12+ (Monterey) |
| Linux | Ubuntu 20.04+, Debian 11+ |
| Windows | WSL2 |

---

## 4. Technical Specifications

### 4.1 Architecture

```
┌─────────────────────────────────────────┐
│              miyabi-cli                  │
│  (CLI entry point, argument parsing)    │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│              miyabi-tui                  │
│  (Terminal UI, Ratatui widgets)         │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│              miyabi-core                 │
│  (LLM client, Tool system, State)       │
└─────────────────────────────────────────┘
```

### 4.2 Dependencies

**Core**:
- tokio: Async runtime
- ratatui: TUI framework
- crossterm: Terminal backend
- clap: CLI parsing

**LLM**:
- reqwest: HTTP client
- serde_json: JSON handling

**Text**:
- pulldown-cmark: Markdown parsing
- syntect: Syntax highlighting

### 4.3 Configuration

**Config File**: `~/.miyabi/config.toml`

```toml
[general]
theme = "dark"
editor = "vim"

[llm]
model = "claude-sonnet-4-20250514"
max_tokens = 4096
temperature = 0.7

[tools]
timeout = 120000
auto_approve = ["read_file", "glob"]
```

---

## 5. User Interface Specifications

### 5.1 Layout

```
┌─────────────────────────────────────────┐
│ miyabi v1.0.0    Tokens: 1.2K/200K  ⚡  │ Header
├─────────────────────────────────────────┤
│                                         │
│  User: How do I implement...            │
│                                         │
│  Assistant: Here's how you can...       │
│  ```rust                                │ Chat History
│  fn main() {                            │
│      println!("Hello");                 │
│  }                                      │
│  ```                                    │
│                                         │
├─────────────────────────────────────────┤
│ > Type your message...                  │ Input
│                                         │
├─────────────────────────────────────────┤
│ Ctrl+C: Cancel  Ctrl+L: Clear  ?: Help  │ Status Bar
└─────────────────────────────────────────┘
```

### 5.2 Color Palette

| Element | Light | Dark |
|---------|-------|------|
| Background | #FFFFFF | #0D1117 |
| Text | #24292F | #C9D1D9 |
| Code | #F6F8FA | #161B22 |
| Accent | #0969DA | #58A6FF |
| Success | #1A7F37 | #3FB950 |
| Error | #CF222E | #F85149 |
| Warning | #9A6700 | #D29922 |

---

## 6. API Specifications

### 6.1 CLI Interface

```bash
miyabi [OPTIONS] [COMMAND]

Commands:
  chat      Start interactive chat (default)
  run       Execute a single prompt
  resume    Resume previous session
  config    Manage configuration
  version   Show version

Options:
  -m, --model <MODEL>    Model to use
  -s, --system <PROMPT>  System prompt
  -v, --verbose          Verbose output
  -h, --help             Show help
```

### 6.2 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | API error |
| 4 | Tool execution error |

---

## 7. Testing Requirements

### 7.1 Unit Tests

- Coverage: > 80%
- All public functions
- Edge cases covered

### 7.2 Integration Tests

- LLM API mocking
- Full conversation flow
- Tool execution

### 7.3 E2E Tests

- Complete user scenarios
- Performance benchmarks

---

## 8. Documentation Requirements

- README.md
- CHANGELOG.md
- API documentation (rustdoc)
- User guide
- Contributing guide

---

## 9. Glossary

| Term | Definition |
|------|------------|
| TUI | Terminal User Interface |
| LLM | Large Language Model |
| SSE | Server-Sent Events |
| Tool | Function callable by LLM |

---

**Document Status**: Draft
**Next Review**: Sprint 1 End
