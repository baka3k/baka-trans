# Observed conventions

The sidecar uses standard-library HTTP classes, dataclasses for immutable artifact metadata, and explicit runtime validation before model access. JSON events are emitted to stdout for lifecycle/progress communication. Request input has explicit byte/text/style/voice bounds and errors use sanitized JSON responses.
