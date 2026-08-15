# Baka Trans — local build & environment helpers.
#
# Targets:
#   make doctor   Verify required toolchain and runtime deps are present.
#   make build    Sync deps and build the desktop app via Tauri.
#
# Notes:
#   - The app reaches a translation model over HTTP via an OpenAI-compatible
#     `/v1/chat/completions` endpoint. `make doctor` probes that endpoint
#     when `LLM_BASE_URL` is set (e.g. `make doctor LLM_BASE_URL=http://localhost:11434/v1`
#     or `export LLM_BASE_URL=...`). The app installs the VieNeu-TTS model
#     on first run, so it is not part of `make build`.
#   - On macOS, ggml (pulled in by whisper-rs) uses std::filesystem::path,
#     which Apple's libc++ marks `@available(macOS 10.15, strict)`. The actual
#     pin lives in `.cargo/config.toml` (`[env] MACOSX_DEPLOYMENT_TARGET = "10.15"`)
#     because cargo does NOT propagate arbitrary parent env vars to build
#     scripts — only what's in `.cargo/config.toml` `[env]` reaches them.
#     `tauri-build` only sets this itself when `bundle.macos.minimum_system_version`
#     is set in tauri config (it isn't), so without the cargo config the
#     value defaults to 10.13 and ggml fails to compile.
#   - cargo's build-script hash does NOT include env vars, so changing
#     `.cargo/config.toml` does not invalidate the cached whisper-rs-sys
#     build (which holds `CMAKE_CXX_FLAGS` from the previous run). The
#     `make build` target depends on `.cargo/.wrs-cleared`, which is
#     regenerated whenever `.cargo/config.toml` is newer, and forces
#     `cargo clean -p whisper-rs-sys` so the next compile picks up the
#     new env. Override per-build with
#     `make build MACOSX_DEPLOYMENT_TARGET=11.0` if your app needs a higher
#     minimum.
#   - Code signing is currently disabled in `tauri.macos.conf.json`
#     (`signingIdentity: null`) so `make build` produces an unsigned `.app`
#     for local smoke-testing. Re-enable when the Apple Development cert is
#     installed in your keychain.

SHELL := /bin/bash

ifeq ($(shell uname -s),Darwin)
    MACOSX_DEPLOYMENT_TARGET ?= 10.15
    BUILD_ENV = MACOSX_DEPLOYMENT_TARGET=$(MACOSX_DEPLOYMENT_TARGET)
else
    BUILD_ENV =
endif

.DEFAULT_GOAL := help

WRS_CLEAR_STAMP := .cargo/.wrs-cleared

.PHONY: help doctor build

help:
	@printf 'Baka Trans — local build targets\n'
	@printf '  make doctor   verify required toolchain and runtime deps\n'
	@printf '  make build     sync deps and build the desktop app\n'

doctor:
	@printf '\n=== Baka Trans — environment check ===\n\n'
	@FAIL=0; \
	check() { \
		if command -v "$$1" >/dev/null 2>&1; then \
			printf '  ✓ %-14s %s\n' "$$1" "$$($$1 $$2 2>/dev/null | head -n1)"; \
		else \
			printf '  ✗ %-14s missing — %s\n' "$$1" "$$3"; \
			FAIL=1; \
		fi; \
	}; \
	printf '[ toolchain ]\n'; \
	check node     --version 'install Node.js 20+ from https://nodejs.org/'; \
	check npm      --version 'reinstall Node.js'; \
	check cargo    --version 'curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'; \
	check rustc    --version 're-run the rustup installer to add rustc'; \
	check cmake    --version 'brew install cmake'; \
	printf '\n[ platform ]\n'; \
	if [ "$$(uname -s)" = "Darwin" ]; then \
		if xcode-select -p >/dev/null 2>&1; then \
			printf '  ✓ %-14s %s\n' xcode-clt "$$(xcode-select -p)"; \
		else \
			printf '  ✗ %-14s missing — xcode-select --install\n' xcode-clt; \
			FAIL=1; \
		fi; \
	else \
		printf '  · %-14s skipped (non-Darwin: %s)\n' xcode-clt "$$(uname -s)"; \
	fi; \
	printf '\n[ runtime services ]\n'; \
	if [ -n "$${LLM_BASE_URL:-}" ]; then \
		LLM_PROBE_URL=$$(echo "$$LLM_BASE_URL" | sed 's:/*$$::')/models; \
		if curl -sf -m 2 "$$LLM_PROBE_URL" >/dev/null 2>&1; then \
			printf '  ✓ %-14s %s\n' 'llm-api' "reachable ($$LLM_PROBE_URL)"; \
		else \
			printf '  ✗ %-14s %s not reachable\n' 'llm-api' "$$LLM_PROBE_URL"; \
			printf '      fix: start the OpenAI-compatible server or set LLM_BASE_URL\n'; \
			FAIL=1; \
		fi; \
	else \
		printf '  · %-14s skipped — set LLM_BASE_URL to probe an OpenAI-compatible API\n' 'llm-api'; \
	fi; \
	printf '\n=== summary ===\n'; \
	if [ "$$FAIL" -ne 0 ]; then \
		printf '✗ environment not ready — fix the items marked ✗ above\n\n'; \
		exit 1; \
	fi; \
	printf '✓ environment looks good\n\n'

build: doctor
ifeq ($(shell uname -s),Darwin)
build: $(WRS_CLEAR_STAMP)
endif
	@printf '\n=== Baka Trans — building desktop app ===\n\n'
	@printf '[1/3] syncing JS deps via npm ci\n'
	$(BUILD_ENV) npm ci
	@printf '\n[2/3] prefetching Rust deps\n'
	$(BUILD_ENV) cargo fetch --manifest-path src-tauri/Cargo.toml
	@printf '\n[3/3] running tauri build\n'
	$(BUILD_ENV) npm run tauri -- build
	@printf '\n✓ build complete — see src-tauri/target/release/bundle/\n'

# Force a fresh whisper-rs-sys build whenever `.cargo/config.toml` is
# edited. cargo's build-script hash doesn't include env vars, so without
# this the next build would reuse stale CMAKE_CXX_FLAGS from the previous
# run and ggml would fail to compile.
$(WRS_CLEAR_STAMP): .cargo/config.toml
	@printf '[pre] clearing stale whisper-rs-sys build cache (env-driven, not tracked by cargo)\n'
	cargo clean -p whisper-rs-sys --manifest-path src-tauri/Cargo.toml
	@mkdir -p $(@D)
	@touch $@

.PHONY: $(WRS_CLEAR_STAMP)
