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
  - Supports standard `~/.kube/config`.
  - Import and manage separate kubeconfigs in `~/.kore/kubeconfigs/`.
  - Instant context switching via Sidebar.
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
├── src/                  # Svelte Frontend
│   ├── lib/
│   │   ├── components/   # UI primitives (DataTable, Sidebar, etc.)
│   │   └── stores/       # Global state (cluster, settings, header)
│   ├── routes/           # File-based routing
│   └── ...
├── src-tauri/            # Rust Backend
│   ├── src/
│   │   ├── config/       # Kubeconfig & App settings management
│   │   ├── k8s.rs        # Kubernetes API logic & Watchers
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

## Configuration

Kore stores its configuration and imported kubeconfigs in:
- **macOS/Linux**: `~/.kore/`
- **Windows**: `C:\Users\<User>\.kore\`

## License

GPL-3.0-or-later
