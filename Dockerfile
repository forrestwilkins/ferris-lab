FROM rust:1.83-slim-bookworm

# Install git and curl
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -s /bin/bash agent
WORKDIR /workspace
RUN chown -R agent:agent /workspace

USER agent

# Pre-warm cargo registry
RUN cargo search --limit 1 serde || true

# Expose WebSocket port
EXPOSE 8080

# Default command runs the agent binary (to be built)
CMD ["cargo", "run", "--release"]
