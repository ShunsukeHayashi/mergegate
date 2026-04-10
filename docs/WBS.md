# Work Breakdown Structure (WBS)

**Project**: mergegate
**Version**: v1.0.0
**Created**: 2025-11-22

---

## 1. TUI Foundation (v0.2.0)

### 1.1 Streaming Markdown Renderer (#2)
- **1.1.1** Core structure and state management (#10) - P0 - 4h
- **1.1.2** Incremental parser with pulldown-cmark (#11) - P0 - 6h
- **1.1.3** Code block handling with syntax highlighting (#12) - P1 - 4h
- **1.1.4** Cursor and scroll position tracking (#13) - P2 - 3h
- **1.1.5** Unit tests (#14) - P2 - 4h

**Subtotal: ~700 lines, 21h**

### 1.2 Git Diff Visualization (#3)
- **1.2.1** Core structure and diff parsing (#15) - P1 - 4h
- **1.2.2** Visualization with colors and indicators (#16) - P1 - 6h
- **1.2.3** Syntax highlighting in diff content (#17) - P2 - 4h
- **1.2.4** Scrollable view with navigation (#18) - P2 - 3h

**Subtotal: ~650 lines, 17h**

---

## 2. Chat Composer (v0.3.0)

### 2.1 Chat Input Component (#4)
- **2.1.1** Separate input handling from main app
- **2.1.2** Multi-line input support
- **2.1.3** Command history navigation
- **2.1.4** Auto-completion
- **2.1.5** Syntax highlighting in input

**Subtotal: ~3,938 lines**

### 2.2 Text Area Widget (#5)
- **2.2.1** Full-featured text area widget
- **2.2.2** Text selection support
- **2.2.3** Copy/paste functionality
- **2.2.4** Undo/redo stack

**Subtotal: ~2,213 lines**

---

## 3. Approval & Navigation (v0.4.0)

### 3.1 Approval Overlay (#6)
- **3.1.1** Tool execution approval UI
- **3.1.2** Y/N/Always/Never options
- **3.1.3** Risk level display
- **3.1.4** Preview of changes

**Subtotal: ~656 lines**

### 3.2 Session Resume (#7)
- **3.2.1** Session resume selection
- **3.2.2** Conversation history browser
- **3.2.3** Search functionality
- **3.2.4** Preview pane

**Subtotal: ~1,580 lines**

---

## 4. LLM Integration (v1.0.0)

### 4.1 Claude API Integration (#8)
- **4.1.1** Anthropic API client with streaming (#19) - P0 - 8h
- **4.1.2** Conversation state and message history (#20) - P0 - 6h
- **4.1.3** Token counting and context window (#21) - P1 - 6h
- **4.1.4** Tool definitions and function calling (#22) - P0 - 8h
- **4.1.5** Rate limiting and retry logic (#23) - P2 - 4h

**Subtotal: ~1,600 lines, 32h**

---

## 5. Tool Execution System (v1.0.0)

### 5.1 Tool System (#9)
- **5.1.1** Tool trait and registry system (#24) - P0 - 6h
- **5.1.2** File operation tools (Read, Write, Edit) (#25) - P0 - 8h
- **5.1.3** Bash tool with sandboxing (#26) - P0 - 8h
- **5.1.4** Search tools (Glob, Grep) (#27) - P1 - 6h
- **5.1.5** Result formatting and error handling (#28) - P2 - 4h

**Subtotal: ~1,800 lines, 32h**

---

## Summary

| Phase | Milestone | Lines | Hours | Issues |
|-------|-----------|-------|-------|--------|
| 1 | v0.2.0 TUI Foundation | 1,350 | 38h | #2, #3, #10-18 |
| 2 | v0.3.0 Chat Composer | 6,151 | - | #4, #5 |
| 3 | v0.4.0 Approval & Nav | 2,236 | - | #6, #7 |
| 4 | v1.0.0 Production | 3,400 | 64h | #8, #9, #19-28 |

**Total: ~13,137+ lines**

---

## Critical Path

```
#19 → #20 → #22 → #24 → #25 → #26 → #27 → #28
 │                 │
 API              Tools
```

**Critical Path Duration: ~42h**

---

## Dependencies Matrix

| Task | Depends On | Blocks |
|------|------------|--------|
| #10 | - | #11 |
| #11 | #10 | #12 |
| #12 | #11 | #13 |
| #13 | #12 | #14 |
| #14 | #13 | - |
| #15 | - | #16 |
| #16 | #15 | #17 |
| #17 | #16 | #18 |
| #18 | #17 | - |
| #19 | - | #20, #23 |
| #20 | #19 | #21, #22 |
| #21 | #20 | - |
| #22 | #20 | - |
| #23 | #19 | - |
| #24 | - | #25, #26, #27 |
| #25 | #24 | #26 |
| #26 | #25 | #27 |
| #27 | #26 | #28 |
| #28 | #27 | - |

---

## Resource Allocation

### Parallel Streams (Max 3)

**Stream A**: TUI (#10-14, #15-18)
**Stream B**: LLM (#19-23)
**Stream C**: Tools (#24-28)

### Sprint Allocation

| Sprint | Stream A | Stream B | Stream C |
|--------|----------|----------|----------|
| 1 | #10, #15 | #19 | #24 |
| 2 | #11, #16 | #20 | #25 |
| 3 | #12, #17 | #21, #22 | #26 |
| 4 | #13, #14, #18 | #23 | #27, #28 |

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| API rate limits | High | Implement caching, retry logic |
| Complex streaming | Medium | Incremental implementation |
| Tool security | High | Sandboxing, input validation |
| Performance | Medium | Benchmarking, profiling |

---

**Last Updated**: 2025-11-22
