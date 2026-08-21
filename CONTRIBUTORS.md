# Contributing

`vibe-kanban-alternative` is maintained as a single-developer project. There is
no formal review board or change-control process — issues and pull requests
are welcome, and the notes below just describe how to make them easy to merge.

## Pull Requests

1. Create a feature or fix branch from `main`.
2. Make focused changes and open a pull request describing the "why".
3. CI (`test.yml`) must pass before merging.

## Coding Practices

### Style & Formatting

- **Rust**: format with `rustfmt` (config in `rustfmt.toml`). Use `snake_case` for modules and functions, `PascalCase` for types. Group imports by crate.
- **TypeScript/React**: pass ESLint and Prettier (2 spaces, single quotes, 80-column width). Use `PascalCase` for components, `camelCase` for variables and functions, `kebab-case` for file names.
- Run `pnpm run format` before submitting a pull request.
- Run `pnpm run lint` to check for lint errors.

### Code Quality

- Keep functions small and focused on a single responsibility.
- Write clear, self-documenting code. Add comments only where the logic is not self-evident.
- Avoid unnecessary abstractions or over-engineering.
- Do not manually edit generated files (e.g. `shared/types.ts`). Modify the source and regenerate.

### Testing

- **Rust**: add unit tests alongside code with `#[cfg(test)]`. Run `cargo test --workspace` to verify.
- **TypeScript**: ensure `pnpm run check` and `pnpm run lint` pass. Include lightweight tests (e.g. Vitest) for new runtime logic.

### Security

- Never commit secrets, credentials, or API keys — use `.env` for local configuration.
- Be mindful of common vulnerabilities (injection, XSS, insecure deserialization) when handling user input or external data.
- Report security issues privately rather than opening a public issue.

### Commit Messages

- Write clear, descriptive commit messages that explain the *why* behind a change.
- Prefix with a conventional type where appropriate (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).
- Keep the subject line under 72 characters; use the body for additional context if needed.
