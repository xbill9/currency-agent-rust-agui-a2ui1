# Makefile for AI Course Creator Web App

# Configuration
SHELL := /bin/bash
PYTHON = python3
PIP = pip3
PORT ?= 8000

# Cloud Run Deployment Variables
PROJECT_ID   ?= $(shell gcloud config get-value project)
REGION       ?= us-central1
SERVICE_NAME ?= course-creator
ORCHESTRATOR_URL = $(shell gcloud run services describe orchestrator --region $(REGION) --format='value(status.url)' --project $(PROJECT_ID) 2>/dev/null)
SERVICE_URL ?= $(shell gcloud run services describe $(SERVICE_NAME) --region $(REGION) --format='value(status.url)' --project $(PROJECT_ID) 2>/dev/null)

# Agent context (for testing)
AGENT_NAME ?= $(SERVICE_NAME)
TOPIC ?= The impact of AI on software engineering

.PHONY: install build-frontend run start stop lint format test check clean help deploy endpoint status check-env a2a test-invoke test-local

help:
	@echo "Available commands:"
	@echo "  install          - Install Python and Frontend dependencies"
	@echo "  build-frontend   - Build the frontend (Vite) to dist/"
	@echo "  run              - Start the FastAPI web app locally"
	@echo "  start            - Start the web app in background"
	@echo "  stop             - Stop the local web app"
	@echo "  lint             - Run ruff to check for linting issues"
	@echo "  format           - Run ruff format and fix"
	@echo "  test             - Run all project tests from the root directory"
	@echo "  check            - Run lint and test"
	@echo "  clean            - Remove temporary files, caches, and build artifacts"
	@echo "  deploy           - Deploy the web app to Cloud Run"
	@echo "  endpoint         - Show the URL of the deployed Cloud Run service"
	@echo "  status           - Show the status of the local and deployed service"
	@echo "  a2a              - Fetch the A2A Agent Card"
	@echo "  test-invoke      - Test the chat endpoint (prefers Cloud Run if deployed)"
	@echo "  test-local       - Test the local chat endpoint"

check-env:
	@if [ -z "$(GOOGLE_API_KEY)" ]; then \
		echo "Warning: GOOGLE_API_KEY is not set. Some backend agents may require it."; \
	fi

install:
	$(PIP) install -r requirements.txt
	cd frontend && npm install

build-frontend:
	cd frontend && npm run build

frontend:
	cd frontend && npm run build

run: check-env
	$(PYTHON) main.py

start: check-env
	@if lsof -Pi :$(PORT) -sTCP:LISTEN -t >/dev/null; then \
		echo "App is already running on port $(PORT)"; \
	else \
		echo "Starting app on port $(PORT)..."; \
		nohup $(PYTHON) main.py > backend.log 2>&1 & echo $$! > app.pid; \
		echo "App started with PID $$(cat app.pid). Logging to backend.log"; \
	fi

stop:
	@echo "Stopping app on port $(PORT)..."
	@if [ -f app.pid ]; then \
		kill $$(cat app.pid) && rm app.pid; \
		echo "App stopped."; \
	elif lsof -Pi :$(PORT) -sTCP:LISTEN -t >/dev/null; then \
		lsof -Pi :$(PORT) -sTCP:LISTEN -t | xargs kill; \
		echo "App stopped."; \
	else \
		echo "App is not running."; \
	fi

lint:
	ruff check .

format:
	ruff format .
	ruff check . --fix

test:
	make -C .. test

test-app:
	python3 -m pytest tests/

check: lint test test-app

deploy:
	@echo "Deploying to Cloud Run ($(SERVICE_NAME)) in project $(PROJECT_ID)..."
	gcloud run deploy $(SERVICE_NAME) \
		--source . \
		--project $(PROJECT_ID) \
		--region $(REGION) \
		--allow-unauthenticated \
		--min-instances 0 \
		--max-instances 1 \
		--set-env-vars AGENT_SERVER_URL=$(ORCHESTRATOR_URL)

endpoint:
	@if [ -z "$(SERVICE_URL)" ]; then \
		echo "Error: Could not determine SERVICE_URL. Ensure the service '$(SERVICE_NAME)' is deployed in project '$(PROJECT_ID)' and region '$(REGION)'."; \
		exit 1; \
	fi
	@echo $(SERVICE_URL)

status:
	@echo "--- Local Status (Port $(PORT)) ---"
	@if lsof -Pi :$(PORT) -sTCP:LISTEN -t >/dev/null; then \
		echo "App is RUNNING on port $(PORT)"; \
	else \
		echo "App is NOT RUNNING locally"; \
	fi
	@echo "--- Cloud Run Status ($(SERVICE_NAME)) ---"
	@gcloud run services describe $(SERVICE_NAME) --region $(REGION) --project $(PROJECT_ID) 2>/dev/null || echo "Service not found on Cloud Run"

clean:
	@echo "Cleaning up caches and temporary files..."
	find . -type d -name "__pycache__" -exec rm -rf {} +
	find . -type d -name ".ruff_cache" -exec rm -rf {} +
	rm -rf frontend/dist
	rm -rf frontend/node_modules
	rm -rf app.pid backend.log
	@echo "Clean completed."

a2a:
	@if [ -z "$(SERVICE_URL)" ]; then echo "Error: SERVICE_URL is not set. Deploy the app first."; exit 1; fi
	@echo "Fetching Agent Card for '$(AGENT_NAME)' from $(SERVICE_URL)..."
	curl -s $(SERVICE_URL)/a2a/$(AGENT_NAME)/.well-known/agent-card.json | jq .

test-invoke:
	@URL=$(if $(SERVICE_URL),$(SERVICE_URL),http://localhost:$(PORT)); \
	echo "Testing chat endpoint at $$URL/api/chat_stream with topic: '$(TOPIC)'..."; \
	curl -s -X POST $$URL/api/chat_stream \
	  -H "Content-Type: application/json" \
	  -d '{"message": "$(TOPIC)", "user_id": "test_user"}' | jq .

test-local:
	@echo "Testing local chat endpoint at http://localhost:$(PORT)/api/chat_stream with topic: '$(TOPIC)'..."; \
	curl -s -X POST http://localhost:$(PORT)/api/chat_stream \
	  -H "Content-Type: application/json" \
	  -d '{"message": "$(TOPIC)", "user_id": "test_user"}' | jq .
