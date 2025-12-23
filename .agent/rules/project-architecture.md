---
trigger: always_on
---

* This is a local LLM chat application using llama.cpp
* Server/client architecture with clear separation
* Server handles llama.cpp integration and exposes an API
* Client is a TUI built with Ratatui
* Prioritize: local-first, privacy, performance
* No external network calls except to local llama.cpp server