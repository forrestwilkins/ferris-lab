# Ferris Lab

A factory for creating CLI utilities with Rust.

## Overview

Ferris Lab is an automated system where two agents work together to build CLI tools with Rust. The agents operate within a sandboxed Docker environment with limited, focused capabilities:

- Write, run, and test Rust code
- Search the web for documentation and solutions
- Collaborate via Git (same machine or over network)
- Communicate through a chat system for coordination and status reporting

## Architecture

The system runs inside a Docker network with two agent containers, each running an instance of `gpt-oss:20b`. Both agents are equal peers that share all responsibilities: writing code, running tests, reviewing each other's work, and fixing issues. They share access to a bare Git repository mounted as a shared volume, allowing them to push and pull changes as they collaborate. A Redis server provides the pub/sub messaging layer for real-time chat between agents, status reporting, and coordination. An oversight dashboard sits outside the sandbox, giving humans visibility into agent activity and the ability to send instructions or intervene when agents get stuck.

## How It Works

1. **Task Assignment** - A CLI tool specification is given to the system
2. **Collaboration** - Both agents work together, dividing tasks as needed through chat
3. **Building** - Agents write code, commit changes, and pull each other's updates
4. **Quality** - Either agent can write tests, review code, and suggest improvements
5. **Delivery** - Completed CLI tool is ready for use

## Getting Started

```bash
# Start the agent infrastructure
docker-compose up -d

# Access the oversight dashboard
open http://localhost:3000
```
