# --- Stage 1: Build Frontend ---
FROM node:25-slim AS frontend-builder
WORKDIR /app/frontend
COPY app/frontend/package*.json ./
RUN npm install
COPY app/frontend/ ./
RUN npm run build

# --- Stage 2: Final Image ---
FROM python:3.13-slim
WORKDIR /app

# Install root dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy all source (including shared, agents, app)
COPY . .

# Copy built frontend from Stage 1 into the app's dist directory
# vite.config.js outDir is '../dist' relative to app/frontend, so it's app/dist
COPY --from=frontend-builder /app/dist /app/app/dist

# Set working directory to app for execution
WORKDIR /app/app

# Environment variables
ENV PYTHONUNBUFFERED=1
ENV HOST 0.0.0.0

# Port for Cloud Run (default is 8080)
EXPOSE 8080

# Run
CMD ["python3", "main.py"]
