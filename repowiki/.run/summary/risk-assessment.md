# Risk assessment

| Risk | Evidence | Mitigation |
| --- | --- | --- |
| Local model availability/latency | local translation requires Whisper, Ollama, and optional VieNeu runtime | test configuration, surface error codes, use smaller models when appropriate |
| Platform-native build prerequisites | README and scripts require toolchains/signing utilities | use CI matrix and release preflight scripts |
| Embedded web CSP disabled | `src-tauri/tauri.conf.json` sets CSP to null | review before any untrusted web content is introduced |
| Credential exposure | cloud key sources include environment and OS keychain | keep key access native and do not log/store literal values |
