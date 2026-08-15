# VieNeu sidecar overview

The sidecar is a managed local Python process that synthesizes Vietnamese speech. It validates the installed artifact manifest and hashes before loading ONNX/int8 model files, then serves the Rust desktop host through authenticated loopback HTTP.

It owns model installation/validation and voice inference. The Rust manager owns lifecycle orchestration, UI-visible state, and audio playback.
