# Ferris Lab

A factory for creating CLI utilities with Rust.

## Overview

Ferris Lab is an automated system where two agents work together to build CLI tools with Rust. The agents operate within a sandboxed Docker environment with limited, focused capabilities:

- Write, run, and test Rust code
- Search the web for documentation and solutions
- Collaborate via Git (same machine or over network)
- Chat with humans via Discord to report status and receive instructions when stuck

## Architecture

The system runs inside a Docker network with two agent containers, each running an instance of `gpt-oss:20b`. Both agents can write code, run tests, review each other's work, and fix issues. They share access to a bare Git repository mounted as a shared volume, allowing them to push and pull changes as they collaborate. Agents communicate with each other and with humans through Discord, where they can report progress, ask for help, and receive instructions.

## How It Works

1. **Task Assignment** - A CLI tool specification is given to the system
2. **Collaboration** - Both agents work together, dividing tasks as needed through chat
3. **Building** - Agents write code, commit changes, and pull each other's updates
4. **Quality** - Either agent can write tests, review code, and suggest improvements
5. **Delivery** - Completed CLI tool is ready for use

## Getting Started

```bash
docker-compose up -d
```
