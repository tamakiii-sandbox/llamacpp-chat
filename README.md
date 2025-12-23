# Local LLM Chat System

A local chat interface for llama.cpp, built with Rust.

## Project Goals

*   **Local-First**: Complete privacy with no external network calls.
*   **Performance**: High-speed TUI and efficient server handling.
*   **Architecture**: Clean separation between the UI Client and the Inference Server.

## Documentation

*   [**Roadmap**](docs/ROADMAP.md): Project status and future plans.
*   [**Architecture**](docs/ARCHITECTURE.md): System design and technology stack.
*   [**Agent Context**](docs/AGENT_CONTEXT.md): Meta-information for AI contributors.

## Architecture

This project consists of three main components:

1.  **llama.cpp HTTP Server**: The inference engine (running separately).
2.  **Rust Server (`server`)**: Acts as a middleware/backend.
    *   Exposes a WebSocket API for the client.
    *   Manages connections to llama.cpp.
    *   Handles message routing and streaming.
3.  **Rust TUI Client (`client`)**: A terminal user interface.
    *   Built with `ratatui`.
    *   Connects to the Rust Server via WebSocket.

## Prerequisites

*   Rust (latest stable)
*   `llama-server` (from llama.cpp) must be in your PATH if you want the server to launch it automatically.
    *   Otherwise, the server will log a warning and run in mock mode (echoing responses).

## Running

1.  Start the server (this will also try to start `llama-server` on port 8080):
    ```bash
    cargo run -p server
    ```

2.  Start the client (in a separate terminal):
    ```bash
    cargo run -p client
    ```

## Development History

For a detailed history of phases, see [docs/ROADMAP.md](docs/ROADMAP.md).