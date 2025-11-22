# Miyabi CLI - Development Goals

## Reference Projects

### Baseline
- **Codex CLI**: https://github.com/openai/codex.git

### Extension Source
- **Miyabi Private**: /Users/shunsuke/dev/01-miyabi/_core/miyabi-private

---

## Mission
**Codex CLI同等の品質を持つ、Production-Ready Miyabi CLIを構築する**

---

## Gap Analysis Summary

| Metric | Codex CLI | Current Miyabi | Target |
|--------|-----------|----------------|--------|
| Total Lines | 20,969 | 1,200 | 15,000+ |
| Files | 50+ | 10 | 40+ |
| Features | Complete | Basic | Full Parity |

---

## Phase 1: Core TUI Foundation ✅ COMPLETED
**Status**: Done (1,200 lines)

- [x] Cargo workspace setup
- [x] wrapping.rs - Text wrapping with textwrap
- [x] history_cell.rs - Card design UI
- [x] markdown_render.rs - Basic markdown
- [x] app.rs - Main TUI loop
- [x] event.rs - Event handling

---

## Phase 2: Advanced Text Rendering 🔄 IN PROGRESS
**Target**: +1,500 lines

### 2.1 markdown_stream.rs (690 lines)
- [ ] Streaming markdown rendering
- [ ] Progressive display during generation
- [ ] Cursor position tracking
- [ ] Partial code block handling

### 2.2 diff_render.rs (715 lines)
- [ ] Git diff visualization
- [ ] Syntax highlighting for diffs
- [ ] Line number display
- [ ] Add/Remove/Change indicators

---

## Phase 3: Chat Composer Separation
**Target**: +4,000 lines

### 3.1 chat_composer.rs (3,938 lines)
- [ ] Separate input handling from main app
- [ ] Multi-line input support
- [ ] Command history
- [ ] Auto-completion
- [ ] Syntax highlighting in input

### 3.2 textarea.rs (2,213 lines)
- [ ] Full-featured text area widget
- [ ] Selection support
- [ ] Copy/paste
- [ ] Undo/redo

---

## Phase 4: Approval & Feedback
**Target**: +1,500 lines

### 4.1 approval_overlay.rs (656 lines)
- [ ] Tool execution approval UI
- [ ] Y/N/Always/Never options
- [ ] Risk level display
- [ ] Preview of changes

### 4.2 feedback_view.rs (555 lines)
- [ ] User feedback collection
- [ ] Rating system
- [ ] Comment input

---

## Phase 5: Advanced Navigation
**Target**: +2,500 lines

### 5.1 resume_picker.rs (1,580 lines)
- [ ] Session resume selection
- [ ] Conversation history browser
- [ ] Search functionality
- [ ] Preview pane

### 5.2 pager_overlay.rs (913 lines)
- [ ] Full-screen content viewer
- [ ] Scroll and search
- [ ] Jump to line
- [ ] Keyboard shortcuts

---

## Phase 6: LLM Integration
**Target**: +2,000 lines

### 6.1 Claude API Integration
- [ ] Anthropic API client
- [ ] Streaming responses
- [ ] Token counting
- [ ] Rate limiting
- [ ] Error handling

### 6.2 Conversation Management
- [ ] Context window management
- [ ] Message history
- [ ] System prompts
- [ ] Tool definitions

### 6.3 Model Coverage & Options
- [ ] Add model presets for `claude-sonnet-4-5-20250929` and `claude-haiku-4-5-20251001`
- [ ] Surface model selection via config/CLI flag
- [ ] Update token/context limits for 4.5 models (context + max output)
- [ ] Handle new stop reason `model_context_window_exceeded`
- [ ] Toggle Extended Thinking (`thinking` param) with sensible defaults

---

## Phase 7: Tool Execution
**Target**: +2,500 lines

### 7.1 A2A Bridge Integration
- [ ] Tool registry
- [ ] Execution engine
- [ ] Result formatting
- [ ] Error recovery

### 7.2 Built-in Tools
- [ ] File operations (Read, Write, Edit)
- [ ] Bash execution
- [ ] Search (Glob, Grep)
- [ ] Web fetch

---

## Success Criteria

### Functional Requirements
- [ ] Real Claude API conversations
- [ ] Tool execution with approval
- [ ] Session persistence and resume
- [ ] Markdown rendering with streaming
- [ ] Full keyboard navigation

### Quality Requirements
- [ ] 15,000+ lines of code
- [ ] 90%+ test coverage
- [ ] < 100ms response latency
- [ ] Memory efficient (< 100MB)
- [ ] Cross-platform (macOS, Linux)

### User Experience
- [ ] Intuitive keyboard shortcuts
- [ ] Clear visual feedback
- [ ] Smooth scrolling
- [ ] Proper error messages
- [ ] Help system

---

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 1 | ✅ Done | Core TUI |
| Phase 2 | 1 week | Streaming Markdown |
| Phase 3 | 2 weeks | Chat Composer |
| Phase 4 | 1 week | Approval System |
| Phase 5 | 1 week | Navigation |
| Phase 6 | 2 weeks | LLM Integration |
| Phase 7 | 2 weeks | Tool Execution |

**Total**: ~10 weeks to Production-Ready

---

## Next Action

**Phase 2.1: Implement markdown_stream.rs**

Create streaming markdown renderer that can display partial content during LLM generation.
