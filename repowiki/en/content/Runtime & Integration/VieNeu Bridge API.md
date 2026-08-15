<cite>
- sidecars/vieneu-tts/server.py
- sidecars/vieneu-tts/pyproject.toml
- src-tauri/src/vieneu.rs
</cite>

# VieNeu Bridge API

## Table of Contents

- [Introduction](#introduction)
- [Endpoints](#endpoints)
- [Model integrity](#model-integrity)
- [Security](#security)

## Introduction

**Verified.** VieNeu is a managed local Python sidecar for Vietnamese speech. Rust starts and supervises it; the bridge binds to loopback and requires a process-provided token and nonce.

## Endpoints

| Operation | Method and path | Result |
| --- | --- | --- |
| Health | `GET /health` | Runtime state and sample-rate metadata |
| Voices | `GET /voices` | Available Vietnamese voice records |
| Synthesis | `POST /synthesize` | WAV audio generated from bounded text, voice, style, rate, and volume |

**Verified.** Requests are authenticated by the bridge handler. The server enforces request-size and text-length limits and validates voices/styles before inference.

## Model integrity

**Verified.** The bridge pins upstream revisions, checks file paths remain under its managed model directory, validates model manifest/version, expected artifact sizes, and SHA-256 digests before loading the runtime.

## Security

**Verified.** The sidecar is not a general network service: its project description and runtime code identify it as loopback-only. Treat its startup token and nonce as secrets; they are deliberately not reproduced here.
