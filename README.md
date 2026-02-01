# Ferris Lab

A factory for creating CLI utilities with Rust.

## Overview

Ferris Lab is an automated system where multiple agents can work together to build CLI tools with Rust. The agents operate within a sandboxed Docker environment with limited, focused capabilities:

- Write, run, and test Rust code
- Search the web for ideas, documentation, and solutions
- Collaborate via Git (same machine or over network)
- Communicate with each other via WebSockets over wifi or the internet
- Chat with users via Discord to report status and receive instructions when they encounter issues

## Architecture

Each agent is a self-contained unit running inside Docker, powered by a model served via Ollama. The default model is `gpt-oss:20b`, but each agent can be configured to use a different model. You can run a single agent on its own, or connect multiple agents across one or more machines. Each agent runs a small web server with a WebSocket endpoint, allowing agents to connect directly to each other for real-time messaging. Agents communicate with users through Discord.

Agents can either roam free, searching the web and coming up with their own ideas for CLI tools, or be steered in a particular direction like database inspection tools or JSON processing utilities.

When multiple agents work together, they vote on which one serves the Git repository while the others maintain backups. If there are only two, they flip a coin instead.

## How It Works

1. **Ideation** - Agents search the web to come up with ideas, or follow a direction given by users
2. **Collaboration** - Agents work together, dividing tasks as needed
3. **Building** - Agents write code, commit changes, and pull each other's updates
4. **Quality** - Either agent can write tests, review code, and suggest improvements
5. **Delivery** - Completed CLI tool is ready for use

## Getting Started

```bash
docker-compose up -d
```

## Running Agents with Cargo

Create a local `.env` file unless you plan to provide all settings via your shell environment:

```bash
cp .env.example .env
```

Run a single agent with defaults (loaded from `.env` if present):

```bash
cargo run
```

Override `.env` values by supplying environment variables inline:

```bash
AGENT_ID=agent-2 AGENT_PORT=8081 PEER_ADDRESSES=ws://localhost:8080/ws cargo run
```

`dotenvy` loads variables from `.env` into the process environment at startup without overriding existing environment variables, so any values you set in your shell take precedence.

## Running Tests

Run a focused test target:

```bash
cargo test --test peer_communication
```
