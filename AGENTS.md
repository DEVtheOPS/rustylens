# Agent Instructions

## Project Context: Kore

**Kore** (Kubernetes Orchestration and Resource Explorer) is a high-performance Kubernetes IDE built as an OpenLens alternative. It leverages Tauri v2 for the backend (Rust) and Svelte 5 (Runes) for the frontend.

### Tech Stack

- **Frontend**: Svelte 5 (Runes based reactivity), Tailwind CSS v4, Lucide Icons.
- **Backend**: Rust (Tauri v2), `kube` crate, `k8s-openapi`.
- **State Management**: Svelte 5 `$state` and `$derived` in `.svelte.ts` store files (e.g., `cluster.svelte.ts`, `settings.svelte.ts`).
- **Styling**: Semantic CSS variables mapped to Tailwind `@theme` (see `src/routes/layout.css`).

### Key Architectural Patterns

1. **Stores**: Centralized logic in `src/lib/stores/`. Use `class` based stores with `$state` fields.
2. **Theming**: Do not hardcode colors. Use semantic variables (`--bg-main`, `--text-muted`, `--color-primary`, etc.).
3. **Kubernetes Interactions**:
    - **Commands**: Simple actions (list, delete) use `#[tauri::command]`.
    - **Streaming**: Resource watching uses Tauri Events (`start_pod_watch` -> `window.emit`).
    - **Config**: Kubeconfigs are aggregated from `~/.kube/config` and `~/.kore/kubeconfigs/`.

### Component Library (`src/lib/components/ui/`)

- **DataTable**: Powerful table with sorting, filtering, column visibility, drag-and-drop, and batch actions.
- **Select/Input/Button/Badge**: Reusable primitives matching the design system.
- **Drawer**: Right-side panel for details.
- **Menu**: Dropdowns for row actions.

---

## Workflow Instructions (Beads)

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

### Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
****
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
