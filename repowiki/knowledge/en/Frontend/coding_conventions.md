# Observed conventions

Files use TypeScript and function components. Public DTO-like types use PascalCase, local utility functions use camelCase, and wire values are string unions. Tauri calls are wrapped rather than issued directly from feature components. Tests sit beside the components or helper modules they exercise and use Vitest with Testing Library.
