# Contributing to Kore

First off, thanks for taking the time to contribute! 🎉

Kore is an open-source Kubernetes IDE built with **Tauri v2** (Rust) and **Svelte 5** (TypeScript). We welcome contributions from everyone—whether you're coding manually or using AI agents.

## 🛠️ Prerequisites

Before you start, ensure you have the following installed:

- **Node.js** (v20+) & **pnpm**
- **Rust** (Stable)
- **Docker** (Optional, for local Kubernetes testing)

## 🚀 Getting Started

1. **Fork and Clone** the repository.
2. **Install Frontend Dependencies**:

    ```bash
    pnpm install
    ```

3. **Install Backend Dependencies**:

    ```bash
    cd src-tauri
    cargo fetch
    cd ..
    ```

4. **Run Development Server**:

    ```bash
    pnpm tauri dev
    ```

## 🏗️ Project Structure

- **`src/`**: Svelte 5 frontend (UI, Stores, Components).
  - `lib/stores/`: State management (Runes).
  - `lib/components/`: Reusable UI components.
  - `routes/`: Application pages and layout.
- **`src-tauri/`**: Rust backend.
  - `src/k8s.rs`: Kubernetes API logic.
  - `src/cluster_manager.rs`: Database and state management.

## 🧪 Testing

We value stability. Please run tests before submitting a PR.

- **Frontend Unit Tests**: `pnpm test:unit`
- **Frontend Coverage**: `pnpm test:coverage`
- **E2E Tests**: `pnpm test` (Playwright)
- **Backend Tests**: `cd src-tauri && cargo test`

## 🤖 Contributing with AI Agents (Cursor, Windsurf, etc.)

We heavily utilize AI in the development of Kore. If you are using an AI agent:

1. **Read the Context**: Make sure your agent reads `AGENTS.md` and `README.md` first. This file contains critical architectural rules and patterns.
2. **Use Svelte 5 Runes**: Ensure your agent generates Svelte 5 code (using `$state`, `$derived`, `$effect`) and **NOT** Svelte 4/3 syntax (stores, `let:` exports).
3. **Follow the Pattern**:
    - **Backend**: Use the macros in `k8s.rs` for new resources.
    - **Frontend**: Use `WorkloadList` component for resource tables.
4. **Verify Code**: AI makes mistakes. Always verify:
    - Imports are correct.
    - Types match between Rust (Backend) and TypeScript (Frontend).
    - Unused variables are removed.
5. **Clean Up**: Don't leave "Todo" comments or placeholders unless necessary.

## 👨‍💻 Contributing Manually

1. **Code Style**:
    - **Rust**: Run `cargo fmt` and `cargo clippy`.
    - **TypeScript**: Run `pnpm check` (svelte-check) and `pnpm lint`.
2. **Commits**: We use [Conventional Commits](https://www.conventionalcommits.org/).
    - `feat: add awesome feature`
    - `fix: resolve crash on startup`
    - `docs: update readme`
3. **Pull Requests**:
    - Describe your changes clearly.
    - Link to any related issues.
    - Ensure CI passes.

## 🤝 Community

Join us in building the fastest Kubernetes IDE! If you have questions, open a [Discussion](https://github.com/DEVtheOPS/kore/discussions) or an Issue.
