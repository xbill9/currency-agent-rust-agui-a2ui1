# AI Course Creator - Web Application

This module provides the web-based interface for the AI Course Creator multi-agent system. It consists of a FastAPI backend and a Vanilla TypeScript frontend.

## Architecture

The web application serves as a bridge between the user and the autonomous agent system:

1.  **Frontend (Vite + TypeScript):** A lightweight, interactive UI that sends user requests to the backend and handles real-time Server-Sent Events (SSE) to display agent progress and the final generated course.
2.  **Backend (FastAPI):** Orchestrates communication with the remote ADK agents using the A2A (Agent-to-Agent) protocol. It proxies streaming responses from the agents and provides session management.

## Key Features

-   **Streaming Progress:** Real-time feedback from specialized `progress_` agents (e.g., "Researcher is gathering information...").
-   **Content Deduplication:** Advanced string merging logic to handle overlapping text fragments from streaming agent outputs.
-   **Session Management:** Persistent session handling for multi-turn interactions with the ADK server.
-   **Cloud-Native:** Built-in support for OpenTelemetry tracing (Google Cloud Trace), JSON logging, and containerized deployment.

## Tech Stack

-   **Backend:** Python 3.12+, FastAPI, Uvicorn, `httpx`, `httpx_sse`.
-   **Frontend:** TypeScript, Vite, CSS, `marked` (for Markdown rendering).
-   **Observability:** OpenTelemetry, Python JSON Logger.

## Configuration

The backend is configured via environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `AGENT_SERVER_URL` | **(Required)** The URL of the ADK agent server (e.g., Orchestrator). | - |
| `AGENT_NAME` | The name of the agent to interact with. | `agent` |
| `PORT` | The port to run the FastAPI server on. | `8080` |
| `LOG_LEVEL` | Logging verbosity (`debug`, `info`, `warning`, `error`). | `info` |

## Development

### Prerequisites

-   Python 3.12+
-   Node.js & npm (for frontend)

### Frontend Build

1.  Navigate to the `frontend/` directory:
    ```bash
    cd frontend
    ```
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Build the production distribution:
    ```bash
    npm run build
    ```
    *This creates a `dist/` directory at the root of the `app` module, which the FastAPI backend serves as static files.*

### Backend Setup

1.  Install Python dependencies:
    ```bash
    pip install -r requirements.txt
    ```
2.  Run the application locally:
    ```bash
    export AGENT_SERVER_URL=http://localhost:8000
    python main.py
    ```

## Testing

The module includes a comprehensive test suite using `pytest`.

```bash
make test
```
