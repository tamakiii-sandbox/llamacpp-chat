# Local LLM Chat System

A local chat interface for llama.cpp, built with Rust.

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
*   A running instance of `llama.cpp` server (for later phases).

## Running

1.  Start the server:
    ```bash
    cargo run -p server
    ```

2.  Start the client (in a separate terminal):
    ```bash
    cargo run -p client
    ```

## Development

*   **Phase 1**: Basic chat (Mocked inference).
*   **Phase 2**: History persistence.
*   **Phase 3**: Hot-switching models.