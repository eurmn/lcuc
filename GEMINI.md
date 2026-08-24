# Project Guidelines & Rules

## Package Manager & Tooling
- Always use `pnpm` instead of `npm` for installing dependencies, running scripts, and managing packages in this repository.
- Do not run `pnpm svelte-check` automatically or proactively without the user explicitly asking for it.
- When the user explicitly requests type-checking Svelte components and TypeScript in this project, use:
  ```bash
  pnpm svelte-check
  ```
- Do not run `cargo check` or `cargo build` proactively before finishing code changes. Only run check/test commands after all code modifications are complete.

