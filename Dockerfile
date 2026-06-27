# Use an official Python runtime as a parent image.
FROM python:3.13-slim

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# Set environment variables
ENV PYTHONUNBUFFERED=1

# Set the working directory in the container
WORKDIR /app

# Copy the current directory contents into the container
COPY . .

# Install dependencies using uv into the system python
RUN uv sync --frozen

# Expose the port the app runs on (default 8080)
EXPOSE 8080

# Run the A2A Agent using uvicorn directly.
CMD ["/app/.venv/bin/uvicorn", "currency_agent.agent:a2a_app", "--host", "0.0.0.0", "--port", "8080"]
