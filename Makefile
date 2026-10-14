# Baka Trans — local build & environment helpers.
#
# Targets:
#   make doctor   Verify required toolchain and runtime deps are present.
#   make build    Sync deps and build the desktop app via Tauri (.app + .dmg).
#   make install  Build, then install the .app into /Applications (macOS).
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
#     `MACOSX_DEPLOYMENT_TARGET` does not invalidate the cached whisper-rs-sys
#     build (which holds `CMAKE_CXX_FLAGS` from the previous run). The
#     `cmake` crate reuses `CMakeCache.txt` if it already exists and skips
#     reconfigure, so the new env var never reaches cmake. To pick up a new
#     value, `make build` shell-checks mtimes and runs
#     `cargo clean -p whisper-rs-sys` (one-time cost ~15 s) when the
#     config is newer than the stamp at `.cargo/wrs-cleared`. It also
#     greps the existing CMakeCache.txt and re-cleans when the cached
#     `CMAKE_OSX_DEPLOYMENT_TARGET` disagrees with the current value.
#     Override per-build with `make build MACOSX_DEPLOYMENT_TARGET=11.0`
#     if your app needs a higher minimum.
#   - Code signing is currently disabled in `tauri.macos.conf.json`
#     (`signingIdentity: null`) so `make build` produces an unsigned `.app`
#     for local smoke-testing. Re-enable when the Apple Development cert is
#     installed in your keychain.
#   - `make install` depends on `build`, then copies the fresh bundle into
#     /Applications/Baka Trans.app: it quits a running instance first,
#     replaces the old bundle, and falls back to sudo when /Applications is
#     not writable by the current user. The unsigned local build carries no
#     quarantine attribute, so it launches straight from /Applications
#     without a Gatekeeper prompt.
#   - Version bumping: `make build BUMP=1` increments the patch version
#     before building; `make build VERSION=x.y.z` sets an exact one (same
#     for `make bump`). Both sync package.json, package-lock.json,
#     tauri.conf.json, Cargo.toml and Cargo.lock through
#     scripts/set-version.mjs. Local bumps run with --force (skipping the
#     "on main + clean tree" release guard); the guarded path for releases
#     stays `npm run version:set <semver>`. The flags are honored from the
#     command line only, so a stray VERSION in the environment cannot
#     hijack a build. The DMG is named after the bumped version, e.g.
#     `Baka Trans_0.4.1_aarch64.dmg`.
#   - Stale hdiutil staging files (`rw.*.dmg`, left behind when a DMG build
#     is interrupted) are removed at the start of `make build` — they can
#     quietly pile up to gigabytes under bundle/macos/.

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

# Bundle paths (keep in sync with productName/identifier in src-tauri/tauri.conf.json).
APP_NAME := Baka Trans
BUNDLE_IDENTIFIER := dev.baka3k.baka-trans
BUNDLE_ROOT := src-tauri/target/release/bundle
APP_BUNDLE := $(BUNDLE_ROOT)/macos/$(APP_NAME).app
DMG_DIR := $(BUNDLE_ROOT)/dmg

.DEFAULT_GOAL := help

WRS_CLEAR_STAMP := .cargo/wrs-cleared

# Version flags are read from the command line only (origin check) so a
# stray VERSION/BUMP exported in the environment cannot hijack a build.
BAKA_SET_VERSION := $(if $(findstring command,$(origin VERSION)),$(strip $(VERSION)),)
BAKA_BUMP := $(if $(findstring command,$(origin BUMP)),$(strip $(BUMP)),)
BUILD_VERSION_PREREQ := $(if $(strip $(BAKA_BUMP)$(BAKA_SET_VERSION)),bump,)

.PHONY: help doctor build install bump

help:
	@printf 'Baka Trans — local build targets\n'
	@printf '  make doctor    verify required toolchain and runtime deps\n'
	@printf '  make build     sync deps and build the desktop app (.app + .dmg)\n'
	@printf '                 BUMP=1 increments the version first; VERSION=x.y.z sets it\n'
	@printf '  make install   build, then install the app into /Applications\n'
	@printf '  make bump      increment the patch version (VERSION=x.y.z to set one)\n'

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

# Local version bump: patch++ by default, VERSION=x.y.z for an exact one.
# Runs set-version.mjs --force — local builds may run on a dirty tree or a
# feature branch; the guarded release path stays `npm run version:set`.
bump:
	@if [ -n "$(BAKA_BUMP)" ] && [ -n "$(BAKA_SET_VERSION)" ]; then \
		printf '✗ pass BUMP=1 or VERSION=x.y.z — not both\n'; \
		exit 1; \
	fi; \
	OLD=$$(node -p "require('./package.json').version"); \
	if [ -n "$(BAKA_SET_VERSION)" ]; then \
		node scripts/set-version.mjs "$(BAKA_SET_VERSION)" --force; \
	else \
		node scripts/set-version.mjs --bump --force; \
	fi; \
	NEW=$$(node -p "require('./package.json').version"); \
	printf '[ver] %s -> %s\n' "$$OLD" "$$NEW"

build: doctor $(BUILD_VERSION_PREREQ)
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		rm -f "$(BUNDLE_ROOT)/macos/"rw.*.dmg 2>/dev/null || true; \
		WRS_CACHE=src-tauri/target/release/build/whisper-rs-sys-*/out/build/CMakeCache.txt; \
		CACHED_OSX=$$(grep -h '^CMAKE_OSX_DEPLOYMENT_TARGET:' $$WRS_CACHE 2>/dev/null | head -1 | cut -d= -f2); \
		if [ ! -f $(WRS_CLEAR_STAMP) ] || [ .cargo/config.toml -nt $(WRS_CLEAR_STAMP) ] || [ "$$CACHED_OSX" != "$(MACOSX_DEPLOYMENT_TARGET)" ]; then \
			if [ -n "$$CACHED_OSX" ] && [ "$$CACHED_OSX" != "$(MACOSX_DEPLOYMENT_TARGET)" ]; then \
				printf '[pre] cached whisper-rs-sys CMakeCache has CMAKE_OSX_DEPLOYMENT_TARGET=%s, want %s — re-clearing\n' "$$CACHED_OSX" "$(MACOSX_DEPLOYMENT_TARGET)"; \
			else \
				printf '[pre] clearing stale whisper-rs-sys build cache (env-driven, not tracked by cargo)\n'; \
			fi; \
			cargo clean -p whisper-rs-sys --manifest-path src-tauri/Cargo.toml; \
			mkdir -p $(@D); \
			touch $(WRS_CLEAR_STAMP); \
		else \
			printf '[pre] whisper-rs-sys cache is fresh (CMAKE_OSX_DEPLOYMENT_TARGET=%s)\n' "$$CACHED_OSX"; \
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
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if [ ! -d "$(APP_BUNDLE)" ]; then \
			printf '✗ expected bundle missing after tauri build: %s\n' "$(APP_BUNDLE)"; \
			exit 1; \
		fi; \
		DMG=$$(ls -t "$(DMG_DIR)"/*.dmg 2>/dev/null | head -n1); \
		if [ -z "$$DMG" ]; then \
			printf '✗ no .dmg produced in %s\n' "$(DMG_DIR)"; \
			exit 1; \
		fi; \
		printf '\n✓ build complete\n'; \
		printf '  app  %s (%s)\n' "$(APP_BUNDLE)" "$$(du -sh "$(APP_BUNDLE)" | cut -f1)"; \
		printf '  dmg  %s (%s)\n' "$$DMG" "$$(du -sh "$$DMG" | cut -f1)"; \
	else \
		printf '\n✓ build complete — see src-tauri/target/release/bundle/\n'; \
	fi

# Install the freshly built bundle into /Applications (macOS only).
# Replaces any previously installed copy; quits a running instance first so
# we never delete a live bundle. sudo is only used when /Applications is not
# writable by the current user.
install: build
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		printf 'make install is macOS-only — it copies the built .app into /Applications\n'; \
		exit 1; \
	fi
	@if [ ! -d "$(APP_BUNDLE)" ]; then \
		printf '✗ %s not found — run make build first\n' "$(APP_BUNDLE)"; \
		exit 1; \
	fi
	@printf '\n=== Installing %s into /Applications ===\n' "$(APP_NAME).app"
	@if pgrep -xq "$(APP_NAME)"; then \
		printf '[pre] quitting running instance of %s\n' "$(APP_NAME)"; \
		osascript -e 'tell application id "$(BUNDLE_IDENTIFIER)" to quit' >/dev/null 2>&1 || true; \
		sleep 1; \
		pkill -x "$(APP_NAME)" 2>/dev/null || true; \
	fi
	@if [ -w /Applications ]; then \
		SUDO=; \
	else \
		SUDO=sudo; \
		printf '[pre] /Applications is not writable by %s — falling back to sudo\n' "$$(whoami)"; \
	fi; \
	$$SUDO rm -rf "/Applications/$(APP_NAME).app"; \
	$$SUDO ditto "$(APP_BUNDLE)" "/Applications/$(APP_NAME).app"; \
	if [ ! -d "/Applications/$(APP_NAME).app" ]; then \
		printf '✗ copy into /Applications failed\n'; \
		exit 1; \
	fi; \
	printf '✓ installed /Applications/%s.app\n' "$(APP_NAME)"; \
	printf '  launch it with: open -a "%s"\n' "$(APP_NAME)"
