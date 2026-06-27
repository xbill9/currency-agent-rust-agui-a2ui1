# React Currency Agent Frontend (CopilotKit)

This is the React frontend UI built with Next.js and CopilotKit, designed to interact with the high-performance compiled **Rust Currency Agent** (running on port 10000).

## Prerequisites

- Node.js 18+
- Rust Currency Agent service running in the background (port 10000)

## Getting Started

1. Install dependencies from the project root using:
   ```bash
   make install
   ```
   *(Or run `npm install` inside this directory).*

2. Set up your environment variable in the root `.env` file containing your `GOOGLE_API_KEY`.

3. Start the React Frontend UI:
   ```bash
   make react-ui
   ```
   *(Or run `npm run dev` inside this directory).*

This will start the application at [http://localhost:3000](http://localhost:3000).

## Key Features

- **Rich Generative UI Components**: Dynamic cards, rate graphs, and visual tables displaying exchange rate intel.
- **Copilot Sidebar Integration**: Interactive copilot chat to query rates, convert currencies, and analyze historical trend data directly.
- **Bypasses Complex Node Agent Backends**: Interfaces directly with the Compiled Rust Agent running on port 10000.

## Troubleshooting

### Agent Connection Issues

If you see connection errors or if the copilot is having trouble fetching rates, make sure:
1. The Rust Currency Agent is running on port 10000 (`make status` to check, `make start` to run).
2. The Rust MCP Server is running on port 8080.
3. Your Google API key is set correctly in `.env`.
