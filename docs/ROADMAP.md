# Project Roadmap

This document corresponds to the project's progress and future plans. It should be updated as major milestones are reached.

## Status: Active Development
**Current Phase**: Phase 3 - Hot-switching models.

---

## Phases

### ✅ Phase 1: Basic Chat
**Goal**: Establish the basic client-server architecture and prove end-to-end message flow.
- [x] Integrate `llama.cpp` server (mocked initially).
- [x] Create Rust server using Actix/Axum/other (decided on: custom `TcpListener` -> `tokio` based).
- [x] Create TUI Client with `ratatui`.
- [x] Implement basic WebSocket communication.
- [x] Verify message flow (User -> Client -> Server -> Llama -> Server -> Client).

### ✅ Phase 2: History Persistence
**Goal**: Save and load conversation history.
- [x] Define storage format (JSON/SQLite).
- [x] Implement history saving on the Server.
- [x] Implement history loading APIs.
- [x] Update Client to display previous history on startup.

### 🔄 Phase 3: Process Management & Model Switching (Current Focus)
**Goal**: Implement a robust Process Manager to control `llama-server` and allow model switching.
- [ ] **Config System**: Create a configuration file (e.g., `models.json`) to map friendly names (e.g., "llama-2") to file paths.
- [ ] **Process Manager**:
    - [ ] Implement robust `spawn` logic with arguments (`-m`, `--port`, `--n-gpu-layers`).
    - [ ] Implement `stop` / `kill` logic to cleanly shut down the old process.
    - [ ] Handle unexpected exits.
- [ ] **Server API**:
    - [ ] Implement `LoadModel` command that triggers the Process Manager restart flow.
- [ ] **Client UI**:
    - [ ] Add Model Selector UI.
    - [ ] Show "Loading..." state during the switch.

### Phase 4: Markdown & UI Polish
**Goal**: Improve the user experience with better text rendering and visual feedback.
- [ ] Implement proper Markdown parsing in `ratatui` (bold, code blocks).
- [ ] Syntax highlighting for code blocks.
- [ ] Improved streaming indicators / cursor animations.
- [ ] Scrollback controls and search.

### Phase 5: Advanced Features (Tentative)
- [ ] **Tool Use**: Allow the model to call basic tools (e.g., calculator, file search).
- [ ] **Multi-modal**: Support for image inputs if `llama.cpp` supports the model.
- [ ] **Session Management**: Support multiple concurrent sessions/conversations.

---
*Last Updated: 2025-12-23*
