# Kore

**Kubernetes Orchestration and Resource Explorer** - A lightweight, open-source Kubernetes IDE built with Tauri v2 and Svelte 5.

![Kore](https://raw.githubusercontent.com/tauri-apps/tauri/dev/.github/splash.png) <!-- Placeholder for actual screenshot -->

## Features

- **🚀 Blazing Fast**: Built on Rust and Tauri, consuming a fraction of the RAM of Electron-based competitors.
- **🎨 Theming System**:
  - **Kore** (Default - Kubernetes Blue)
  - **Kore Light**
  - **Dracula**
  - **Alucard** (Light Dracula)
  - **Rusty** & **Rusty Light** (Legacy)
- **☸️ Multi-Cluster Management**:
  - Import kubeconfigs from files or folders with automatic context extraction.
  - Each cluster stored independently with UUID-based routing.
  - SQLite database for cluster metadata (name, icon, description, tags).
  - Bookmark favorite clusters in the icon sidebar for quick access.
  - Drag-and-drop to reorder bookmarks.
- **⚡ Real-time Updates**: Kubernetes resources update in real-time using efficient watch streams.
- **📊 Advanced Data Tables**:
  - Sorting, Filtering, and Column Reordering.
  - Multi-selection and Batch Actions (e.g., Bulk Delete).
  - Persistent user preferences for column visibility.
- **🛠️ Workload Management**: View, Edit, Log, Shell, and Delete Pods (more resources coming soon).

## Tech Stack

- **Frontend Framework**: [Svelte 5](https://svelte.dev/) (Runes)
- **Desktop Framework**: [Tauri v2](https://v2.tauri.app/)
- **Styling**: [Tailwind CSS v4](https://tailwindcss.com/)
- **Kubernetes Client**: `kube-rs` & `k8s-openapi`
- **Icons**: `lucide-svelte`

## Project Structure

```
├── src/                         # Svelte Frontend
│   ├── lib/
│   │   ├── components/
│   │   │   ├── ui/              # Reusable UI components
│   │   │   ├── IconSidebar.svelte    # Left-most navigation
│   │   │   ├── ResourceSidebar.svelte # Cluster resource navigation
│   │   │   └── ClusterImportModal.svelte
│   │   └── stores/
│   │       ├── clusters.svelte.ts     # Cluster CRUD operations
│   │       ├── activeCluster.svelte.ts # Current cluster state
│   │       ├── bookmarks.svelte.ts    # Sidebar bookmarks
│   │       └── settings.svelte.ts     # App settings
│   ├── routes/
│   │   ├── +page.svelte              # Cluster overview
│   │   ├── cluster/[id]/             # Cluster-scoped routes
│   │   │   ├── pods/
│   │   │   ├── deployments/
│   │   │   ├── settings/             # Cluster settings
│   │   │   └── ...
│   │   └── settings/                 # App settings
│   └── ...
├── src-tauri/            # Rust Backend
│   ├── src/
│   │   ├── cluster_manager.rs # SQLite cluster storage
│   │   ├── import.rs          # Kubeconfig import & extraction
│   │   ├── k8s.rs             # Kubernetes API & Watchers
│   │   └── ...
│   └── ...
```

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) & [pnpm](https://pnpm.io/)
- Docker (optional, for local k8s testing)

### Setup

1. **Install dependencies**:
   ```bash
   pnpm install
   cd src-tauri && cargo fetch
   ```

2. **Run Development Server**:
   ```bash
   pnpm tauri dev
   ```

### Building for Production

```bash
pnpm tauri build
```

### Running Tests & Coverage

**Frontend (Svelte/TS)**

```bash
# Run Unit Tests
pnpm test:unit

# Run Unit Tests with Coverage
pnpm test:coverage

# Run Playwright E2E Tests
pnpm test
```

**Backend (Rust)**

```bash
# Run Unit Tests
cd src-tauri
cargo test

# Run Coverage (requires cargo-llvm-cov)
# Install: cargo install cargo-llvm-cov
cargo llvm-cov
```

See [tests/README.md](tests/README.md) for more details.

## Configuration

Kore stores its configuration in:
- **macOS/Linux**: `~/.kore/`
- **Windows**: `C:\Users\<User>\.kore\`

Storage structure:
```
~/.kore/
├── clusters.db              # SQLite database (cluster metadata)
├── kubeconfigs/             # Extracted single-context configs
│   ├── <uuid-1>.yaml
│   ├── <uuid-2>.yaml
│   └── ...
└── bookmarks.json           # Sidebar bookmarks
```

## License

GPL-3.0-or-later
