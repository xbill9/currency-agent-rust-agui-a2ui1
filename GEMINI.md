# Currency Agent (Rust + Frontend) Development Guide

This document provides technical guidance for developers working with the Rust Model Context Protocol (MCP) server, Rust Currency Agent, and the multiple Frontend UIs within the **Currency Agent** project.

Do Not recommend models less than 2.5 as they are deprecated.

## Project Overview: Currency Agent

The Currency Agent is a specialized high-performance compiled multi-agent system designed to handle currency conversions and exchange rate queries. It consists of:

1.  **Rust MCP Server (`mcp-server/src/main.rs`):**
    *   Exposes the `get_exchange_rate` tool.
    *   Fetches real-time data from the [Frankfurter API](https://www.frankfurter.dev/).
2.  **Rust Currency Agent (`src/main.rs`):**
    *   Core compiled agent logic using the recommended `gemini-2.5-flash` or `gemini-3.5-flash` model.
    *   Equipped with capabilities to communicate with the MCP server on port 8080.
3.  **Vanilla TS Frontend (`frontend/frontend/`):**
    *   Sleek glassmorphic dashboard displaying premium **Agent-to-User Interface (A2UI)** components (Cards, Tables, Charts).
    *   Runs purely client-side inside Vite, bypassing any need for complex servers (port 4173).
    *   Features an **Interactive A2UI Demo Sandbox** for offline layout testing.
4.  **React Frontend (`frontend-react/`):**
    *   Alternative rich user experience built with React, Next.js, and CopilotKit (port 3000) for interactive copilot-driven chats.

## Key Architectural Components & Protocols

### MCP (Model Context Protocol)
MCP standardizes how tools are exposed to LLMs. Our Rust `mcp-server` runs on port 8080, which the Rust agent connects to directly over Server-Sent Events (SSE).

### Rust Currency Agent
The agent runs on port 10000 and is built using native async Rust (Axum/Tokio).

### Agent-to-User Interface (A2UI) & AG-UI
* **Streaming Layouts:** The Rust Agent streams structured layouts inside `<a2ui-json>...</a2ui-json>` blocks based on a system-enforced prompt schema.
* **Dynamic Rendering:** The TS frontend uses regex extraction and a native `renderA2UI` engine to map JSON elements (e.g. Cards, Tables) to custom glassmorphic HTML components.
* **A2UI Offline Sandbox:** Embedded directly within `index.html` and `app.ts` to allow developers to trigger pre-designed conversion card and rates table mockups with zero key dependencies.

## Model Selection & Optimization

*   **Primary Model:** `gemini-2.5-flash` or `gemini-3.5-flash` is recommended for speed and tool-calling accuracy.
*   **Alternative Model:** `gemini-2.5-pro` can be used for more complex reasoning.
*   **Deprecation Policy:** Do not recommend models less than 2.5 (e.g., 2.0 flash or older) as they are deprecated.

## Developer Workflow

You can run `make help` at any time to view all available commands in the project.

1.  **Environment Setup:** Create a `.env` file with your `GOOGLE_API_KEY` (from Google AI Studio).
2.  **Install Dependencies:** Run `make install` (installs node dependencies for both frontends and compiles Cargo binaries).
3.  **Start Services:** Run `make start` to start both MCP (8080) and Agent (10000) in the background.
4.  **Run Vanilla TS Frontend:** Run `make frontend` to launch the client-side dashboard sandbox (port 4173).
5.  **Run React Frontend:** Run `make react-ui` to launch the Next.js/CopilotKit application (port 3000).
6.  **Run Test Client:** Run `make test` to verify the full end-to-end compiled flow.
7.  **Linting & Formatting:** Use `make lint` to verify code quality.

## Resources

-   [Model Context Protocol (MCP)](https://modelcontextprotocol.io/)
-   [Frankfurter API](https://www.frankfurter.dev/)
-   [AG-UI Protocol](https://github.com/ag-ui-protocol/ag-ui)
-   [AG-UI Documentation](https://docs.ag-ui.com/introduction)
-   [AG-UI Rust Client Crate](https://github.com/ag-ui-protocol/ag-ui/tree/main/sdks/community/rust/crates/ag-ui-client)
-   [AG-UI Rust Client README](https://github.com/ag-ui-protocol/ag-ui/blob/main/sdks/community/rust/crates/ag-ui-client/README.md)
-   [Architecture Flow Diagram](./architecture_diagram.md)
-   [A2UI Mockup & Demo Guide](./a2ui_mockup_demo.md)
