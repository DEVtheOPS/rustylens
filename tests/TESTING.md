# Kore Testing Documentation

## Test Coverage Overview

Kore has comprehensive test coverage split between backend unit tests and frontend E2E tests.

## Backend Tests (Rust)

### Import Module Tests (`src-tauri/src/import.rs`)

Unit tests for cluster import and kubeconfig discovery functionality.

```bash
# Run all backend tests
cd src-tauri && cargo test

# Run only import tests
cd src-tauri && cargo test import::tests

# Run with output
cd src-tauri && cargo test import::tests -- --nocapture
```

#### Test Cases:

1. **`test_discover_contexts_in_file`** ✅
   - Tests discovering multiple contexts from a single kubeconfig file
   - Validates context names, cluster names, and user names are correctly parsed

2. **`test_discover_contexts_in_folder`** ✅
   - Tests recursive folder scanning for multiple kubeconfig files
   - Validates all contexts across multiple files are discovered
   - Tests subdirectory traversal

3. **`test_extract_context`** ✅
   - Tests extracting a single context from a multi-context kubeconfig
   - Validates context parsing logic

4. **`test_invalid_kubeconfig`** ✅
   - Tests error handling for malformed YAML files
   - Ensures graceful failure with appropriate error messages

5. **`test_nonexistent_file`** ✅
   - Tests error handling for missing files
   - Validates proper error propagation

6. **`test_empty_folder`** ✅
   - Tests scanning empty directories
   - Validates empty result set is returned correctly

### What Backend Tests Cover:
- ✅ Kubeconfig file parsing
- ✅ Multi-context discovery
- ✅ Recursive folder scanning
- ✅ Context extraction
- ✅ Error handling (invalid YAML, missing files)
- ✅ Edge cases (empty folders)

### What Backend Tests DON'T Cover (by design):
- ❌ Tauri command invocation (requires running app)
- ❌ Database operations (tested separately in cluster_manager)
- ❌ File system watchers (removed feature)

## Frontend Tests (Playwright)

### E2E UI Tests (`tests/*.spec.ts`)

End-to-end tests for the user interface.

```bash
# Run all E2E tests
pnpm test

# Run with UI
pnpm test:ui

# Run specific test file
pnpm test tests/01-layout-navigation.spec.ts

# Run in headed mode (see browser)
pnpm test --headed
```

#### Test Suites:

1. **Layout & Navigation** (`01-layout-navigation.spec.ts`)
   - Icon sidebar visibility
   - Navigation between pages
   - Theme application
   - Responsive layout

2. **Icon Sidebar** (`02-icon-sidebar.spec.ts`)
   - Active state indicators
   - Add cluster button
   - Import modal opening/closing
   - Bookmark display

3. **Cluster Overview** (`03-cluster-overview.spec.ts`)
   - Empty state handling
   - DataTable display
   - Search and refresh
   - Column configuration

4. **Cluster Import** (`04-cluster-import.spec.ts`)
   - Modal structure and tabs
   - File/folder import UI
   - Modal closing behavior
   - Tab switching

5. **Settings** (`05-settings.spec.ts`)
   - Settings page structure
   - Theme selector
   - Theme persistence

6. **Cluster Routes** (`06-cluster-routes.spec.ts`)
   - Route structure validation
   - Error state handling
   - URL preservation

### What Frontend Tests Cover:
- ✅ UI component rendering
- ✅ Navigation flows
- ✅ Modal interactions
- ✅ Visual structure
- ✅ Theme application
- ✅ Empty states

### What Frontend Tests DON'T Cover (limitations):
- ❌ Actual file selection (native OS dialogs can't be automated)
- ❌ Complete import flow (requires real kubeconfig files)
- ❌ Kubernetes API calls (requires real clusters)

## Integration Testing Strategy

For features that are difficult to test with unit or E2E tests:

### Manual Testing Checklist

1. **Import Flow**
   - [ ] Import single kubeconfig file
   - [ ] Import folder with multiple files
   - [ ] Import file with multiple contexts
   - [ ] Verify display names can be customized
   - [ ] Verify icons can be set
   - [ ] Verify import button validation works

2. **Cluster Management**
   - [ ] Create cluster
   - [ ] Update cluster metadata
   - [ ] Delete cluster
   - [ ] Bookmark/unbookmark clusters
   - [ ] Reorder bookmarks via drag-and-drop

3. **Kubernetes Operations** (requires real cluster)
   - [ ] List namespaces
   - [ ] List pods
   - [ ] View pod details
   - [ ] Stream pod logs
   - [ ] Delete pod
   - [ ] Real-time watch updates

## Test Data

### Sample Kubeconfig (for manual testing)

Create `~/.kore/test-config.yaml`:

```yaml
apiVersion: v1
kind: Config
current-context: test-context
clusters:
- name: test-cluster
  cluster:
    server: https://test.example.com
    insecure-skip-tls-verify: true
users:
- name: test-user
  user:
    token: test-token-12345
contexts:
- name: test-context
  context:
    cluster: test-cluster
    user: test-user
    namespace: default
```

## CI/CD Integration

### GitHub Actions (recommended setup)

```yaml
name: Tests

on: [push, pull_request]

jobs:
  backend-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run backend tests
        run: cd src-tauri && cargo test

  frontend-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: pnpm/action-setup@v2
        with:
          version: 8
      - name: Install dependencies
        run: pnpm install
      - name: Install Playwright
        run: npx playwright install chromium
      - name: Run E2E tests
        run: pnpm test
```

## Coverage Reports

### Backend Coverage (Rust)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cd src-tauri && cargo tarpaulin --out Html
```

### Frontend Coverage (Playwright)

Playwright automatically generates HTML reports on test failure.

```bash
# View last test report
npx playwright show-report
```

## Test Philosophy

1. **Unit tests** validate logic in isolation
2. **E2E tests** validate user-facing behavior
3. **Manual testing** covers complex integrations (native dialogs, real clusters)

This three-tiered approach ensures:
- Core logic is thoroughly tested
- User experience is validated
- Real-world scenarios are verified before release

## Running All Tests

```bash
# Backend tests
cd src-tauri && cargo test

# Frontend tests
pnpm test

# Type checking
pnpm run check
```

All tests should pass before merging code! 🧪✅
