# Global Defaults

- Keep answers concise, technical, and implementation-focused
- Prefer functional, declarative patterns and modular solutions
- Favor named exports for functions and components
- Use kebab-case for directories and file names
- Match existing project patterns before introducing new abstractions
- Respect documented boundaries in `docs/`: do not modify PRD/proposal intent, implement against it
- Backend stack assumptions: Rust services (Axum, Postgres)
- Frontend stack assumptions: React-based desktop UI surface

# Product and Protocol Alignment

- Treat SPS-50, SPS-51, and SPS-65 as source-of-truth protocol references
- Keep strict separation of multisig authorities across backend and frontend logic
- Preserve manual fallback paths (users can still aggregate signatures and broadcast if backend is unavailable)
- Prioritize signer safety: explicit confirmation steps, authority context, and high-signal error messages

# Formatting and Naming

- Use tabs for indentation
- Use single quotes unless escaping makes double quotes clearer
- Omit semicolons unless required for correctness
- Use strict equality (`===`) instead of loose equality (`==`)
- Keep line length around 120 characters
- Use descriptive boolean names (`isLoading`, `hasError`, `canSubmit`)
