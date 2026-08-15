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
#     or `export LLM_BASE_URL=...`). `make build` also builds the bundled
#     VieNeu-TTS bridge (scripts/build-vieneu-sidecar.sh) and the managed
#     Hy-MT runtime sidecar (scripts/build-hy-mt-sidecar.sh), both PyInstaller,
#     so the packaged app ships the managed runtimes. The VieNeu and Hy-MT
#     models themselves are downloaded in-app on first run; they are not part
#     of `make build`.
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
#     build (which holds `CMAKE_CXX_FLAGS` from the previous run). To
#     pick up a new value, `make build` shell-checks mtimes and runs
#     `cargo clean -p whisper-rs-sys` (one-time cost ~15 s) when the
#     config is newer than the stamp at `.cargo/wrs-cleared`. Override
#     per-build with `make build MACOSX_DEPLOYMENT_TARGET=11.0` if your
#     app needs a higher minimum.
#   - Code signing is currently disabled in `tauri.macos.conf.json`
#     (`signingIdentity: null`) so `make build` produces an unsigned `.app`
#     for local smoke-testing. Re-enable when the Apple Development cert is
#     installed in your keychain.

SHELL := /bin/bash

ifeq ($(shell uname -s),Darwin)
    MACOSX_DEPLOYMENT_TARGET ?= 10.15
    BUILD_ENV = MACOSX_DEPLOYMENT_TARGET=$(MACOSX_DEPLOYMENT_TARGET)
    VIENEU_SIDECAR = scripts/build-vieneu-sidecar.sh
    HYMT_SIDECAR = scripts/build-hy-mt-sidecar.sh
else
    BUILD_ENV =
    VIENEU_SIDECAR = @printf 'skipped (non-Darwin: build via scripts/build-vieneu-sidecar.ps1)\n'
    HYMT_SIDECAR = @printf 'skipped (non-Darwin: build via scripts/build-hy-mt-sidecar.ps1)\n'
endif

.DEFAULT_GOAL := help

WRS_CLEAR_STAMP := .cargo/wrs-cleared

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
		if command -v uv >/dev/null 2>&1; then \
			printf '  ✓ %-14s %s\n' uv "$$(uv --version 2>/dev/null)"; \
		else \
			printf '  ✗ %-14s missing — required by sidecar builds (https://docs.astral.sh/uv/)\n' uv; \
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
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if [ ! -f $(WRS_CLEAR_STAMP) ] || [ .cargo/config.toml -nt $(WRS_CLEAR_STAMP) ]; then \
			printf '[pre] clearing stale whisper-rs-sys build cache (env-driven, not tracked by cargo)\n'; \
			cargo clean -p whisper-rs-sys --manifest-path src-tauri/Cargo.toml; \
			mkdir -p $(@D); \
			touch $(WRS_CLEAR_STAMP); \
		else \
			printf '[pre] whisper-rs-sys cache is fresh\n'; \
		fi \
	fi
	@printf '\n=== Baka Trans — building desktop app ===\n\n'
	@printf '[1/5] syncing JS deps via npm ci\n'
	$(BUILD_ENV) npm ci
	@printf '\n[2/5] prefetching Rust deps\n'
	$(BUILD_ENV) cargo fetch --manifest-path src-tauri/Cargo.toml
	@printf '\n[3/5] building the bundled VieNeu-TTS bridge\n'
	$(BUILD_ENV) $(VIENEU_SIDECAR)
	@printf '\n[4/5] building the managed Hy-MT runtime sidecar\n'
	$(BUILD_ENV) $(HYMT_SIDECAR)
	@printf '\n[5/5] running tauri build\n'
	$(BUILD_ENV) npm run tauri -- build
	@printf '\n✓ build complete — see src-tauri/target/release/bundle/\n'
