# Test Coverage Analysis

## Executive Summary

The Kore codebase has **88 existing tests** spread across **9 of 21 modules**. The testing
strategy follows a three-tier approach (unit tests, E2E tests, manual tests) but has significant
gaps in unit test coverage — particularly on the frontend where only 1 of 7 stores has any
tests, and on the backend where the largest modules (`k8s/pod.rs`, `k8s/workload.rs`,
`k8s/statefulset.rs`) have zero tests.

**Overall coverage by module count:** 9/21 modules tested (43%)

---

## Current Test Inventory

### Backend (Rust) — 82 unit tests across 6 modules

| Module | Tests | What's Covered |
|--------|------:|----------------|
| `k8s/deployment.rs` | 54 | Event filtering, struct serialization, pod mapping, conditions |
| `k8s/metrics.rs` | 10 | CPU/memory quantity parsing (all unit formats) |
| `import.rs` | 8 | Context discovery, folder scanning, depth limits, symlinks |
| `input_validation.rs` | 6 | Name/description/tag validation, trimming, deduplication |
| `config/mod.rs` | 4 | Path traversal prevention, file permissions, symlink escape |
| `cluster_manager.rs` | 3 | Invalid name rejection, duplicate tags, invalid description |
| `image_utils.rs` | 3 | Image resize, aspect ratio, base64 encoding |

### Frontend (TypeScript) — 2 unit tests in 1 module

| Module | Tests | What's Covered |
|--------|------:|----------------|
| `stores/header.svelte.ts` | 2 | Store initialization and title update |

### E2E (Playwright) — ~45 test cases across 6 spec files

| Spec File | Tests | What's Covered |
|-----------|------:|----------------|
| `01-layout-navigation` | 8 | Layout structure, navigation, theming |
| `02-icon-sidebar` | 9 | Active states, add button, bookmarks display |
| `03-cluster-overview` | 8 | Page structure, DataTable columns, search/refresh buttons |
| `04-cluster-import` | 11 | Modal structure, tabs, open/close behavior |
| `05-settings` | 9 | Settings page, theme selector, navigation |
| `06-cluster-routes` | 8 | Route validation, non-existent cluster handling |

---

## Modules With Zero Tests

### Backend — High Priority

| Module | Lines | Complexity | Public API |
|--------|------:|------------|------------|
| `k8s/pod.rs` | ~600 | Very High | 13 commands, 10+ structs, `map_pod_to_summary` |
| `k8s/workload.rs` | ~800 | Very High | 40+ commands, 20+ mapping functions |
| `k8s/statefulset.rs` | ~300 | High | 3 commands, 3 structs, event filtering |
| `k8s/client.rs` | ~200 | High | Client creation, context resolution |
| `k8s/common.rs` | ~80 | Medium | `calculate_age`, `get_created_at` |
| `lib.rs` | ~128 | Low | Tauri setup, state management |

### Frontend — High Priority

| Module | Complexity | Public API |
|--------|------------|------------|
| `stores/clusters.svelte.ts` | Medium | CRUD operations, IPC calls, error recovery |
| `stores/activeCluster.svelte.ts` | Medium | Cluster switching, namespace fetching, storage sync |
| `stores/bookmarks.svelte.ts` | Medium | Add/remove/toggle/reorder, localStorage persistence |
| `stores/bottomDrawer.svelte.ts` | Medium | Tab lifecycle, callbacks, state coordination |
| `stores/settings.svelte.ts` | Low | Theme management, localStorage persistence |
| `stores/cluster.svelte.ts` | Medium | Context refresh, namespace fetching |

---

## Recommendations — Prioritized

### Priority 1: Backend Pure-Function Unit Tests (High Impact, Low Effort)

These functions have complex logic, no external dependencies, and can be tested immediately
without mocking Kubernetes APIs.

#### 1A. `k8s/common.rs` — `calculate_age` and `get_created_at`

These are used by every resource type in the application. They have date-math logic that is
a classic source of bugs.

**Suggested tests:**
- Timestamps from seconds ago → "45s"
- Timestamps from minutes ago → "30m"
- Timestamps from hours ago → "2h"
- Timestamps from days ago → "5d"
- Boundary values: exactly 60s → should show "1m" not "60s"
- Boundary values: exactly 24h → should show "1d" not "24h"
- Future timestamps (clock skew)
- None/missing timestamp handling
- `get_created_at` with valid and None timestamps

#### 1B. `k8s/pod.rs` — `map_pod_to_summary` and `probe_to_info`

`map_pod_to_summary` is the most complex mapping function in the codebase. It extracts
container states, restart counts, resource requests/limits, environment variables, probes,
and ports from deeply nested Kubernetes Pod objects.

**Suggested tests:**
- Running pod with 1 container, healthy
- Pod with multiple containers in mixed states (Running, Waiting, Terminated)
- Pod in CrashLoopBackOff (waiting with reason)
- Pod in Pending state with no node assignment
- Pod with init containers
- Container with full resource requests and limits
- Container with probes (liveness, readiness, startup) — tests `probe_to_info`
- Container with environment variables from ConfigMap/Secret references
- Pod with volume mounts
- Pod with no containers (edge case)
- Container restart count aggregation across multiple containers

#### 1C. `k8s/workload.rs` — All `map_*_to_summary` functions

There are 14+ mapping functions here, one per Kubernetes resource type. Each extracts
status information into a `WorkloadSummary`.

**Suggested tests (for each resource type):**
- `map_deployment_to_summary` — ready/total replicas, status derivation
- `map_statefulset_to_summary` — ready/total replicas
- `map_daemonset_to_summary` — desired/ready/available counts
- `map_job_to_summary` — completion status, active/succeeded/failed counts
- `map_cronjob_to_summary` — schedule, last schedule time, active job count
- `map_service_to_summary` — type, cluster IP, ports
- `map_ingress_to_summary` — rules, hosts
- `map_configmap_to_summary` — data key count
- `map_secret_to_summary` — data key count, type
- `map_node_to_summary` — conditions, capacity

Each mapping function should be tested with:
- A fully-populated resource
- A minimal resource (only required fields)
- Edge cases specific to that resource type

#### 1D. `k8s/statefulset.rs` — `map_pod_to_statefulset_pod_info` and `filter_statefulset_events`

These are structurally similar to the deployment equivalents (which are well-tested with 54
tests), but have zero tests themselves.

**Suggested tests:** Mirror the deployment.rs test patterns:
- Pod mapping with Running/Pending/Failed states
- Event filtering by name, kind, and UID
- Event sorting by timestamp
- Struct serialization round-trips

### Priority 2: Frontend Store Unit Tests (High Impact, Medium Effort)

The frontend stores manage all application state and have complex logic around localStorage
persistence, Tauri IPC calls, and cross-store coordination. They can be tested with Vitest
by mocking the Tauri `invoke` function.

#### 2A. `stores/bookmarks.svelte.ts`

This store has the most testable pure logic: array manipulation, ordering, deduplication.

**Suggested tests:**
- `add()` assigns correct order (max existing order + 1)
- `remove()` deletes correct bookmark
- `isBookmarked()` returns true/false correctly
- `toggle()` switches between bookmarked and not
- `reorder()` correctly swaps items and updates order fields
- `getBookmarkedClusterIds()` returns sorted by order
- `load()` restores from localStorage
- `save()` persists to localStorage
- Empty state: no bookmarks, operations don't crash
- Adding duplicate bookmark (same clusterId)

#### 2B. `stores/bottomDrawer.svelte.ts`

Tab lifecycle management with callbacks is logic-heavy and bug-prone.

**Suggested tests:**
- `openTab()` adds new tab and sets it active
- `openTab()` with existing tab just activates it (no duplicate)
- `closeTab()` removes tab and fires `onClose` callback
- `closeTab()` on last tab closes the drawer
- `setActiveTab()` switches to correct tab
- `activeTab` getter returns the right tab
- `toggle()` opens/closes drawer
- `close()` clears active state

#### 2C. `stores/settings.svelte.ts`

**Suggested tests:**
- Default theme is applied on initialization
- `setTheme()` updates the theme property
- `setRefreshInterval()` updates interval
- `save()` persists to localStorage
- Loading from localStorage restores previous settings
- Invalid localStorage data doesn't crash

#### 2D. `stores/clusters.svelte.ts` and `stores/activeCluster.svelte.ts`

These require mocking `@tauri-apps/api/core` invoke calls but are critical paths.

**Suggested tests (clusters):**
- `load()` populates the clusters list from backend
- `get()` retrieves a single cluster
- `update()` sends correct data to backend
- `remove()` deletes cluster and refreshes list
- `getTags()` parses JSON tags correctly
- Error handling when backend calls fail

**Suggested tests (activeCluster):**
- `setCluster()` switches context and fetches namespaces
- `setNamespace()` updates namespace and persists
- `loadFromStorage()` restores previous state
- `contextName` getter derives name correctly
- Setting cluster to null clears state

### Priority 3: Expanding `cluster_manager.rs` Tests (Medium Impact, Low Effort)

The cluster manager has only 3 tests but performs critical database operations.

**Suggested tests:**
- `add_cluster` with valid data → cluster created with UUID, timestamps set
- `list_clusters` returns clusters sorted by last_accessed descending
- `get_cluster` returns None for non-existent ID
- `update_cluster` with partial updates (only name, only icon, etc.)
- `update_last_accessed` changes the timestamp
- `delete_cluster` removes record and associated config file
- `delete_cluster` with non-existent ID → graceful error
- Database initialization creates table with correct schema
- Concurrent operations don't corrupt data

### Priority 4: E2E Test Functional Coverage (Medium Impact, High Effort)

The current E2E tests verify UI **structure** (elements exist, correct classes) but don't
test **behavior** (clicking search actually filters, refresh reloads data, sorting works).

**Suggested additions:**
- DataTable sorting: click column headers, verify row order changes
- DataTable search: type in search box, verify rows are filtered
- DataTable column visibility: toggle columns, verify they appear/disappear
- Theme change: select a different theme, verify CSS variables change
- Theme persistence: change theme, reload page, verify theme persists
- Settings page: change refresh interval, verify it takes effect
- Cluster detail navigation: navigate through resource sidebar links

### Priority 5: Error Path Testing (Low Priority, Medium Effort)

Most tests cover the happy path. Error scenarios are largely untested.

**Backend error paths to test:**
- `k8s/client.rs` — Invalid context name, missing kubeconfig, expired certificates
- `import.rs` — Permission denied on file read, corrupted YAML, empty contexts
- `cluster_manager.rs` — Database locked, disk full, corrupt database file
- `image_utils.rs` — Corrupted image file, unsupported format, zero-byte file

**Frontend error paths to test:**
- Tauri IPC call failures (backend unavailable)
- localStorage quota exceeded
- Invalid JSON in localStorage

---

## Quantified Gap Summary

| Area | Existing Tests | Estimated Missing | Priority |
|------|---------------:|------------------:|----------|
| `k8s/pod.rs` mapping functions | 0 | ~30 | P1 |
| `k8s/workload.rs` mapping functions | 0 | ~40 | P1 |
| `k8s/common.rs` utilities | 0 | ~10 | P1 |
| `k8s/statefulset.rs` mapping/filtering | 0 | ~25 | P1 |
| Frontend stores (6 untested) | 0 | ~50 | P2 |
| `cluster_manager.rs` CRUD | 3 | ~10 | P3 |
| E2E behavioral tests | 0 | ~15 | P4 |
| Error path coverage | ~5 | ~25 | P5 |
| **Totals** | **88** | **~205** | |

Closing the P1 and P2 gaps alone would bring the test count from 88 to ~243 and cover
all modules with complex logic.

---

## Quick Wins (Can Be Done Immediately)

1. **`k8s/common.rs` — 10 tests for `calculate_age`**: Pure function, no dependencies, tests
   can be written in minutes. Every resource type depends on this function.

2. **`stores/bookmarks.svelte.ts` — 10 tests for array logic**: Pure state management with
   no IPC dependencies. Only needs localStorage mocking.

3. **`k8s/statefulset.rs` — Copy deployment.rs test patterns**: The code structure mirrors
   `deployment.rs` which already has 54 tests. The same test patterns can be adapted.

4. **`stores/settings.svelte.ts` — 6 tests**: Simplest store with the least dependencies.

5. **`k8s/workload.rs` — `map_deployment_to_summary`**: Start with one mapping function.
   The pattern can then be replicated across the other 13 resource types.
