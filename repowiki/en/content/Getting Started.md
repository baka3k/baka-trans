<cite>
- README.md
- package.json
- src-tauri/Cargo.toml
</cite>

# Getting Started

## Table of Contents

- [Introduction](#introduction)
- [Prerequisites](#prerequisites)
- [Run locally](#run-locally)
- [Local translation](#local-translation)

## Introduction

**Verified.** Baka Trans is a Tauri desktop application for real-time meeting translation on macOS and Windows. The development entry point starts a Vite frontend and the Rust desktop host together.

## Prerequisites

**Verified.** Install Node.js (CI uses Node 22), npm, Rust, CMake, and a C/C++ toolchain. macOS builds need Xcode command-line tools; Windows builds need Visual Studio Build Tools with desktop C++ support. Local spoken translation additionally uses Ollama and a Whisper model.

## Run locally

From the repository root:

```bash
npm ci
npm run tauri -- dev
```

**Verified.** `npm ci` consumes `package-lock.json`; the `tauri` script runs the Tauri CLI, whose `beforeDevCommand` is `npm run dev`.

## Local translation

**Verified.** The local path is `PCM16/16 kHz → Whisper → Ollama /api/chat → Vietnamese TTS → selected output`. It is intended for Japanese-to-Vietnamese in the current release. Choose Local mode, configure or download a Whisper model, choose an Ollama model, then test the pipeline before starting a session.
