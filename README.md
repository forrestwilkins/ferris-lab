# Ferris Lab

A factory for creating CLI utilities with Rust.

## Overview

Ferris Lab is an automated system where two agents work together to build CLI tools with Rust. The agents operate within a sandboxed Docker environment with limited, focused capabilities:

- Write, run, and test Rust code
- Search the web for ideas, documentation, and solutions
- Collaborate via Git (same machine or over network)
- Communicate with each other via Redis pub/sub over wifi or the internet
- Chat with humans via Discord to report status and receive instructions when stuck

## Architecture

Each agent is a self-contained unit running `gpt-oss:20b` inside Docker, and can be deployed on separate machines. Agents communicate with each other via Redis pub/sub and with humans through Discord. When two agents start working together, they flip a coin to decide which one serves the Git repository while the other maintains a backup as they make progress.

## How It Works

1. **Ideation** - Agents search the web to come up with ideas for useful CLI tools
2. **Collaboration** - Both agents work together, dividing tasks as needed
3. **Building** - Agents write code, commit changes, and pull each other's updates
4. **Quality** - Either agent can write tests, review code, and suggest improvements
5. **Delivery** - Completed CLI tool is ready for use

## Getting Started

```bash
docker-compose up -d
```
