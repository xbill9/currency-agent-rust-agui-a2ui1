# Currency Agent Architecture & Protocols (with A2UI)

Below is a visual flow of interactions between the user-facing workspace frontend, the compiled Rust Agent, the Gemini LLM, the local Model Context Protocol (MCP) server, and the external data source:

![Architecture Diagram](/home/xbill/currency-agent-rust/images/architecture_diagram_a2ui.png)

## Architecture Diagram (Mermaid)

Below is the Mermaid flowchart representation of the architecture:

```mermaid
graph TD
    %% Styling
    classDef client fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:#0d47a1;
    classDef agent fill:#f1f8e9,stroke:#558b2f,stroke-width:2px,color:#33691e;
    classDef llm fill:#ede7f6,stroke:#651fff,stroke-width:2px,color:#4a148c;
    classDef mcp fill:#fff3e0,stroke:#ef6c00,stroke-width:2px,color:#e65100;
    classDef external fill:#fafafa,stroke:#9e9e9e,stroke-width:2px,color:#212121;
    classDef ui fill:#f3e5f5,stroke:#8e24aa,stroke-width:2px,color:#7b1fa2;

    %% Nodes
    Browser["User Web Browser<br/>(Currency Intellect Frontend)"]:::client
    A2UIEngine["A2UI Rendering Engine<br/>(renderA2UI in app.ts)"]:::ui
    AgentServer["Rust A2A Agent Server<br/>(src/main.rs)"]:::agent
    A2UISchema["A2UI Schema Manager<br/>(SYSTEM_INSTRUCTION in src/main.rs)"]:::ui
    Gemini["Google Gemini API<br/>(gemini-2.5-flash)"]:::llm
    MCPServer["Rust MCP Server<br/>(mcp-server/src/main.rs)"]:::mcp
    FrankAPI["Frankfurter API<br/>(api.frankfurter.dev)"]:::external

    %% Connections and Protocols
    Browser -->|"1. A2A Protocol / HTTP<br/>(Port 10000 - proxied)"| AgentServer
    AgentServer -->|"2. Gemini API / HTTPS<br/>(With A2UI instructions)"| Gemini
    AgentServer -->|"3. Model Context Protocol / SSE<br/>(Port 8080)"| MCPServer
    MCPServer -->|"4. HTTP REST / JSON"| FrankAPI

    %% Response cycle
    FrankAPI -.->|Rates JSON| MCPServer
    MCPServer -.->|Tool Output| AgentServer
    AgentServer -.->|Updated Context| Gemini
    Gemini -.->|"5. Text response with A2UI block"| AgentServer
    AgentServer -.->|"6. Delivery to Browser<br/>(A2A Protocol / JSON response)"| Browser
    Browser -->|"7. Pass JSON to engine"| A2UIEngine
    A2UIEngine -.->|"8. Render dynamic cards/tables"| Browser

    subgraph Frontend Application
        Browser
        A2UIEngine
    end

    subgraph Rust Agent Ecosystem
        AgentServer
        A2UISchema
    end

    subgraph Model Context Protocol
        MCPServer
    end
```

---

## Detailed Components & Protocols

### 1. Agent-to-Agent (A2A) Protocol
*   **Source:** [app.ts](file:///home/xbill/currency-agent-rust/frontend/frontend/app.ts)
*   **Destination:** [main.rs](file:///home/xbill/currency-agent-rust/src/main.rs)
*   **Port:** Default `10000` (or `8080` in Cloud Run env)
*   **Protocol Details:** 
    *   Vite dev server proxies `/api` requests to the compiled Axum-based Rust Agent Server.
    *   Uses standard HTTP/JSON payloads to register, exchange, and resolve agent metadata cards.
    *   Supports conversation context propagation using `contextId`.

### 2. Google Gemini API Protocol
*   **Source:** [main.rs](file:///home/xbill/currency-agent-rust/src/main.rs)
*   **Destination:** Google GenAI Gemini endpoint
*   **Protocol Details:**
    *   Secured via `GOOGLE_API_KEY` (Gemini API) or Google Cloud Service Account credentials (Vertex AI).
    *   Uses standard HTTPS JSON payloads.
    *   Communicates model instructions, user prompts, conversation history, and handles function/tool-calling schemas returned by the model.
    *   The primary model used is `gemini-2.5-flash`.

### 3. Model Context Protocol (MCP)
*   **Source:** [main.rs](file:///home/xbill/currency-agent-rust/src/main.rs)
*   **Destination:** [main.rs](file:///home/xbill/currency-agent-rust/mcp-server/src/main.rs)
*   **Port:** Default `8080` (endpoint `/mcp`)
*   **Protocol Details:**
    *   Standardizes tool exposure and execution schema under the **Model Context Protocol** spec.
    *   Communicates over HTTP using Server-Sent Events (SSE).
    *   The MCP Server advertises the available tool (`get_exchange_rate`) and receives tool execution requests containing parameters (e.g. `currency_from`, `currency_to`, `currency_date`).

### 4. HTTP REST/JSON API Protocol
*   **Source:** [main.rs](file:///home/xbill/currency-agent-rust/mcp-server/src/main.rs)
*   **Destination:** Frankfurter API (`https://api.frankfurter.dev`)
*   **Protocol Details:**
    *   Standard HTTP GET request using standard Rust async client (`reqwest`).
    *   Fetches real-time market data in standard JSON format containing exchange rates.

### 5. Agent-to-UI (A2UI) Protocol & Integration
*   **Source:** `SYSTEM_INSTRUCTION` schema definition inside [main.rs](file:///home/xbill/currency-agent-rust/src/main.rs)
*   **Destination:** [app.ts](file:///home/xbill/currency-agent-rust/frontend/frontend/app.ts) (runs in user's browser)
*   **Protocol Details:**
    *   Defines standard JSON schemas for rich, interactive, and responsive UI components (e.g. Cards, Tables, Text fields, Columns, Rows).
    *   The Rust Agent Server injects system prompt instructions instructing the Gemini model on component schemas.
    *   The model generates structured layout descriptions wrapped in `<a2ui-json>` elements.
    *   The TS frontend client parses responses via regex for `<a2ui-json>...</a2ui-json>`, then uses the `renderA2UI()` engine to map JSON components directly to glassmorphic, stylized HTML container elements.

