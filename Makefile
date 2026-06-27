# Makefile for Currency Agent (Rust + Frontend)

# Environment variables for local development
export GOOGLE_GENAI_USE_VERTEXAI ?= False
export LOG_LEVEL ?= INFO
# Gemini 2.5 Flash / Gemini 3.5 Flash are recommended
export GENAI_MODEL ?= gemini-2.5-flash
export MCP_SERVER_URL ?= http://127.0.0.1:8080/mcp

.PHONY: help install mcp agent frontend frontend-install frontend-build react-install react-ui start stop status test test-rust-e2e lint format clean deploy logs endpoint remote-status

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "Services:"
	@echo "  start            - Start background Rust services (MCP + Agent)"
	@echo "  stop             - Stop background Rust & Frontend services"
	@echo "  status           - Check status of running services"
	@echo "  mcp              - Start the Rust MCP Server (foreground)"
	@echo "  agent            - Start the Rust Currency Agent (foreground)"
	@echo ""
	@echo "Vanilla TS Frontend (Port 4173):"
	@echo "  frontend-install - Install dependencies for Vanilla TS Frontend"
	@echo "  frontend-build   - Build production assets for Vanilla TS Frontend"
	@echo "  frontend         - Build and preview the Vanilla TS Frontend"
	@echo ""
	@echo "React Frontend (Port 3000):"
	@echo "  react-install    - Install dependencies for React Frontend"
	@echo "  react-ui         - Start React Frontend UI"
	@echo ""
	@echo "Development & Quality:"
	@echo "  install          - Install all Rust & Frontend dependencies"
	@echo "  test             - Run Rust end-to-end tests (MCP + Agent + Test Client)"
	@echo "  test-rust-e2e    - Cleanly compile, launch, and run end-to-end client tests"
	@echo "  lint             - Run cargo clippy and format checks"
	@echo "  format           - Auto-format Rust code"
	@echo "  clean            - Remove logs and caches"
	@echo ""
	@echo "Cloud Deployment:"
	@echo "  deploy           - Deploy to Cloud Run using Cloud Build"
	@echo "  logs             - Read logs from Cloud Run"
	@echo "  endpoint         - Get the Cloud Run service endpoint"
	@echo "  remote-status    - Check the status of the remote endpoint"

install:
	@echo "Installing Node.js dependencies..."
	$(MAKE) frontend-install
	$(MAKE) react-install
	@echo "Building Rust projects..."
	cargo build
	cargo build --manifest-path mcp-server/Cargo.toml

start:
	@echo "Building Rust projects..."
	@cargo build --manifest-path mcp-server/Cargo.toml > /dev/null
	@cargo build --bin currency-agent > /dev/null
	@echo "Starting Rust MCP Server in background..."
	@nohup ./mcp-server/target/debug/mcp-server > mcp.log 2>&1 &
	@echo "Waiting for MCP Server to initialize..."
	@sleep 2
	@echo "Starting Rust Currency Agent in background..."
	@nohup ./target/debug/currency-agent > agent.log 2>&1 &
	@echo "Services started. Logs: mcp.log, agent.log"

stop:
	@echo "Stopping servers..."
	@pgrep -f "target/debug/mcp-server" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@pgrep -f "cargo run --manifest-path mcp-server" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@pgrep -f "target/debug/currency-agent" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@pgrep -f "cargo run --bin currency-agent" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@pgrep -f "vite preview" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@pgrep -f "next-dev" | grep -v "$$$$" | xargs kill -9 2>/dev/null || true
	@sleep 1

status:
	@echo "Checking status of services..."
	@PID_MCP=$$(pgrep -f "target/debug/mcp-server" | tr '\n' ' '); \
	if [ -n "$$PID_MCP" ]; then \
		echo "  MCP Server:       Running on port 8080 (PID $$PID_MCP)"; \
	else \
		echo "  MCP Server:       Stopped (Port 8080)"; \
	fi
	@PID_AGENT=$$(pgrep -f "target/debug/currency-agent" | tr '\n' ' '); \
	if [ -n "$$PID_AGENT" ]; then \
		echo "  Currency Agent:   Running on port 10000 (PID $$PID_AGENT)"; \
	else \
		echo "  Currency Agent:   Stopped (Port 10000)"; \
	fi
	@PID_VITE=$$(pgrep -f "vite preview" | tr '\n' ' '); \
	if [ -n "$$PID_VITE" ]; then \
		echo "  Vite Preview:     Running on port 4173 (PID $$PID_VITE)"; \
	else \
		echo "  Vite Preview:     Stopped (Port 4173)"; \
	fi
	@PID_REACT=$$(pgrep -f "next dev" | tr '\n' ' '); \
	if [ -n "$$PID_REACT" ]; then \
		echo "  React Frontend:   Running on port 3000 (PID $$PID_REACT)"; \
	else \
		echo "  React Frontend:   Stopped (Port 3000)"; \
	fi

test: test-rust-e2e

mcp:
	@echo "Starting MCP Server on port 8080..."
	cargo run --manifest-path mcp-server/Cargo.toml

agent:
	@echo "Starting Rust Currency Agent on port 10000..."
	cargo run --bin currency-agent

frontend: frontend-build
	@echo "Starting Frontend Preview Server..."
	cd frontend/frontend && npm run preview

frontend-install:
	@echo "Installing frontend dependencies..."
	cd frontend/frontend && npm install

frontend-build:
	@echo "Building frontend..."
	cd frontend/frontend && npm run build

react-install:
	@echo "Installing React frontend dependencies..."
	cd frontend-react && npm install

react-ui:
	@echo "Starting React Frontend UI (port 3000)..."
	cd frontend-react && npm run dev:ui

test-rust-e2e:
	@echo "Running Rust end-to-end test..."
	-$(MAKE) stop
	@echo "Starting Rust MCP Server (port 8080)..."
	@cd mcp-server && nohup cargo run --bin mcp-server > ../mcp_rust.log 2>&1 &
	@sleep 2
	@echo "Starting Rust Currency Agent (port 10000)..."
	@nohup cargo run --bin currency-agent > agent_rust.log 2>&1 &
	@sleep 3
	@echo "Running Rust Test Client..."
	-cargo run --bin test_client
	$(MAKE) stop

lint:
	@echo "Running Rust linting checks (clippy + fmt)..."
	cargo clippy --all-targets -- -D warnings
	cargo clippy --manifest-path mcp-server/Cargo.toml --all-targets -- -D warnings
	cargo fmt --all -- --check
	cargo fmt --manifest-path mcp-server/Cargo.toml --all -- --check

format:
	@echo "Auto-formatting Rust code..."
	cargo fmt --all
	cargo fmt --manifest-path mcp-server/Cargo.toml --all

clean:
	@echo "Cleaning up caches and logs..."
	rm -rf .ruff_cache
	rm -f *.log
	@echo "Clean completed."

deploy:
	@echo "Deploying to Cloud Run via Cloud Build..."
	gcloud builds submit --config cloudbuild.yaml .

logs:
	@echo "Reading Cloud Run service logs..."
	gcloud run services logs read currency-agent --region us-central1 --limit 5

endpoint:
	@echo "Getting Cloud Run service endpoint..."
	@gcloud run services describe currency-agent --region us-central1 --format "value(status.url)"

remote-status:
	@echo "Checking remote endpoint status..."
	@ENDPOINT=$$(gcloud run services describe currency-agent --region us-central1 --format "value(status.url)"); \
	curl -s -o /dev/null -w "%{http_code}" $$ENDPOINT/health || echo "Failed to connect"
