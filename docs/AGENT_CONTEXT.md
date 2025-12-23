# Agent Context & Handover

**This file is for the AI Agent.** 
If you are an AI reading this, this is your "Anchor". It describes the meta-state of the project that cannot be captured in code alone.

## Project Identity
**Project Name**: Local LLM Chat System (llama.cpp + Rust)
**Core Value**: A high-performance, private, local chat experience using a split client/server architecture.

## Current Focus (Meta-State)
*   **Active Phase**: Phase 3 - Process Management.
*   **Immediate Goal**: We need to be able to launch `llama-server` with specific arguments (model path, CPU/GPU layers) before we can "switch" models.
*   **User Preferences**:
    *   OS: Linux.
    *   Architecture: `server` handles logic, `client` is view-only.
    *   **Always** split work into Planning -> Execution -> Verification tasks using `task_boundary`.
    *   **Evergreen Docs**: Keep this file and `docs/` updated as the first step of any major transition.

## Memory Bank / "Gotchas"
*   **`llama-server` Management**: Currently `server/src/main.rs` just spawns a process blindly. This needs to be replaced with a `ProcessManager` struct that manages the child process handle and can kill/restart it.
*   **WebSocket Protocol**: We are using a simple JSON message format over WebSocket. Check `shared/src/lib.rs` for the exact schema.
*   **Ratatui**: We use `ratatui` for the TUI. Remember that `ratatui` renders to an alternate screen buffer, so standard `println!` debugging might not be visible. Use logging (trace/debug files) instead.

## Next Steps (Handover)
*   Complete the Phase 3 tasks in `docs/ROADMAP.md`.
*   Specifically, `server/src/main.rs` needs to be updated to accept a "LoadModel" command over the WebSocket or HTTP control plane.
*   `client/src/ui.rs` (or equivalent) needs a new modal or command to select the model.
