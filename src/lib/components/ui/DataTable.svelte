<script lang="ts">
  import { ArrowUp, ArrowDown, GripVertical, Settings2, Eye, EyeOff, RefreshCw, Search } from "lucide-svelte";
  import Button from "./Button.svelte";
  import Input from "./Input.svelte";
  import Menu, { type MenuItem } from "./Menu.svelte";
  import Checkbox from "./Checkbox.svelte";
  import type { Snippet } from "svelte";

  export interface BatchAction {
    label: string;
    action: (selectedIds: any[]) => void;
    icon?: any;
    danger?: boolean;
  }

  export interface Column {
    id: string;
    label: string;
    sortable?: boolean;
    visible?: boolean;
    width?: string;
    sortKey?: string;
  }

  let {
    data = $bindable([]),
    columns = $bindable([]),
    keyField = "id",
    children, // cell snippet
    onRowClick,
    storageKey,
    search = $bindable(""),
    onRefresh,
    loading = false,
    showSearch = true,
    showRefresh = true,
    actions,
    batchActions,
  }: {
    data: any[];
    columns: Column[];
    keyField?: string;
    children?: Snippet<[{ row: any; column: Column; value: any }]>;
    onRowClick?: (row: any) => void;
    storageKey?: string;
    search?: string;
    onRefresh?: () => void;
    loading?: boolean;
    showSearch?: boolean;
    showRefresh?: boolean;
    actions?: (row: any) => MenuItem[];
    batchActions?: BatchAction[];
  } = $props();

  let sortCol = $state<string | null>(null);
  let sortDir = $state<"asc" | "desc">("asc");
  let draggingCol = $state<string | null>(null);
  let showConfig = $state(false);
  let selectedIds = $state<Set<any>>(new Set());

  // Handle Select All
  function toggleSelectAll(checked: boolean) {
    if (checked) {
      selectedIds = new Set(data.map((r) => r[keyField]));
    } else {
      selectedIds = new Set();
    }
  }

  // Handle Row Selection
  function toggleRow(id: any, checked: boolean) {
    const newSet = new Set(selectedIds);
    if (checked) {
      newSet.add(id);
    } else {
      newSet.delete(id);
    }
    selectedIds = newSet;
  }

  const allSelected = $derived(data.length > 0 && selectedIds.size === data.length);
  const isIndeterminate = $derived(selectedIds.size > 0 && selectedIds.size < data.length);
  const showSelection = $derived(batchActions && batchActions.length > 0);

  // Initialize from storage - runs once on mount
  let initialized = false;
  $effect(() => {
    if (initialized) return;
    initialized = true;

    if (storageKey && typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(`datatable-${storageKey}`);
      if (saved) {
        try {
          const savedCols = JSON.parse(saved);
          // Merge saved visibility with current columns
          columns = columns.map((c) => {
            const savedCol = savedCols.find((sc: any) => sc.id === c.id);
            if (savedCol && typeof savedCol.visible !== "undefined") {
              return { ...c, visible: savedCol.visible };
            }
            return c;
          });
        } catch (e) {
          console.error("Failed to load table settings", e);
        }
      }
    }
  });

  // Sorting Logic
  function handleSort(colId: string) {
    const col = columns.find((c) => c.id === colId);
    if (!col?.sortable) return;

    if (sortCol === colId) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortCol = colId;
      sortDir = "asc";
    }
  }

  const sortedData = $derived(
    [...data].sort((a, b) => {
      if (!sortCol) return 0;
      const col = columns.find((c) => c.id === sortCol);
      if (!col) return 0;

      const key = col.sortKey || sortCol;
      const valA = a[key];
      const valB = b[key];

      // Handle undefined/null values
      if (valA === undefined || valA === null) return sortDir === "asc" ? 1 : -1;
      if (valB === undefined || valB === null) return sortDir === "asc" ? -1 : 1;

      // Simple string/number comparison
      if (valA < valB) return sortDir === "asc" ? -1 : 1;
      if (valA > valB) return sortDir === "asc" ? 1 : -1;
      return 0;
    }),
  );

  const visibleColumns = $derived(columns.filter((c) => c.visible !== false));

  // Clear selection when data changes (optional, but safer)
  $effect(() => {
    // If data items disappear, remove them from selection
    const currentIds = new Set(data.map((r) => r[keyField]));
    const newSelected = new Set([...selectedIds].filter((id) => currentIds.has(id)));
    if (newSelected.size !== selectedIds.size) {
      selectedIds = newSelected;
    }
  });

  // Drag and Drop Logic
  function onDragStart(e: DragEvent, colId: string) {
    draggingCol = colId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function onDrop(e: DragEvent, targetColId: string) {
    e.preventDefault();
    if (!draggingCol || draggingCol === targetColId) return;

    const fromIdx = columns.findIndex((c) => c.id === draggingCol);
    const toIdx = columns.findIndex((c) => c.id === targetColId);

    if (fromIdx !== -1 && toIdx !== -1) {
      const newCols = [...columns];
      const [moved] = newCols.splice(fromIdx, 1);
      newCols.splice(toIdx, 0, moved);
      columns = newCols;
    }
    draggingCol = null;
  }

  function toggleColumn(colId: string) {
    columns = columns.map((c) =>
      c.id === colId ? { ...c, visible: c.visible === undefined ? false : !c.visible } : c,
    );
  }

  $effect(() => {
    if (storageKey && typeof localStorage !== "undefined") {
      const settings = columns.map((c) => ({ id: c.id, visible: c.visible }));
      localStorage.setItem(`datatable-${storageKey}`, JSON.stringify(settings));
    }
  });
</script>

<div class="h-full flex flex-col space-y-4">
  <!-- Toolbar -->
  {#if showSearch || showRefresh}
    <div class="flex items-center justify-end gap-3">
      {#if showSearch}
        <div class="relative w-64">
          <Search class="absolute left-2.5 top-2.5 text-text-muted" size={16} />
          <Input placeholder="Search..." class="pl-9" bind:value={search} />
        </div>
      {/if}

      {#if showRefresh && onRefresh}
        <Button variant="secondary" onclick={onRefresh} disabled={loading}>
          <RefreshCw size={16} class={loading ? "animate-spin" : ""} />
        </Button>
      {/if}

      <!-- Config -->
      <div class="relative">
        <Button variant="ghost" size="sm" onclick={() => (showConfig = !showConfig)}>
          <Settings2 size={16} />
        </Button>

        {#if showConfig}
          <div
            class="absolute right-0 top-full mt-1 w-48 bg-bg-popover border border-border-subtle rounded-md shadow-lg z-50 p-2"
          >
            <div class="text-xs font-semibold text-text-muted mb-2 px-2">Columns</div>
            {#each columns.filter((c) => c.id !== "actions") as col (col.id)}
              <button
                class="flex items-center w-full px-2 py-1 text-sm hover:bg-bg-panel rounded text-left gap-2"
                onclick={() => toggleColumn(col.id)}
              >
                {#if col.visible !== false}
                  <Eye size={14} class="text-primary" />
                {:else}
                  <EyeOff size={14} class="text-text-muted" />
                {/if}
                <span class="flex-1 truncate">{col.label}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Table -->
  <div class="flex-1 overflow-auto rounded-md border border-border-subtle bg-bg-card">
    <table class="w-full text-sm text-left">
      <thead
        class="bg-bg-panel text-text-muted font-semibold border-b border-border-subtle sticky top-0 z-10 shadow-sm"
      >
        <tr>
          {#if showSelection}
            <th class="px-4 py-3 w-[40px]">
              <Checkbox checked={allSelected} indeterminate={isIndeterminate} onchange={toggleSelectAll} />
            </th>
          {/if}
          {#each visibleColumns as col (col.id)}
            <th
              class="px-4 py-3 relative group select-none {col.sortable ? 'cursor-pointer hover:text-text-main' : ''}"
              draggable="true"
              ondragstart={(e) => onDragStart(e, col.id)}
              ondragover={onDragOver}
              ondrop={(e) => onDrop(e, col.id)}
              onclick={() => handleSort(col.id)}
              style="width: {col.width}"
            >
              <div class="flex items-center gap-2">
                <GripVertical size={12} class="opacity-0 group-hover:opacity-50 cursor-grab" />
                <span>{col.label}</span>
                {#if sortCol === col.id}
                  {#if sortDir === "asc"}
                    <ArrowUp size={14} />
                  {:else}
                    <ArrowDown size={14} />
                  {/if}
                {/if}
              </div>
            </th>
          {/each}
          {#if actions}
            <th class="px-4 py-3 w-[50px]"></th>
          {/if}
        </tr>
      </thead>
      <tbody class="divide-y divide-border-subtle">
        {#each sortedData as row (row[keyField])}
          <tr
            class="hover:bg-bg-panel/50 transition-colors {onRowClick ? 'cursor-pointer' : ''} {selectedIds.has(
              row[keyField],
            )
              ? 'bg-primary/5'
              : ''}"
            onclick={() => onRowClick?.(row)}
          >
            {#if showSelection}
              <td class="px-4 py-3" onclick={(e) => e.stopPropagation()}>
                <Checkbox
                  checked={selectedIds.has(row[keyField])}
                  onchange={(checked) => toggleRow(row[keyField], checked)}
                />
              </td>
            {/if}
            {#each visibleColumns as col (col.id)}
              <td class="px-4 py-3 text-text-main">
                {#if children}
                  {@render children({ row, column: col, value: row[col.id] })}
                {:else}
                  {row[col.id]}
                {/if}
              </td>
            {/each}
            {#if actions}
              <td class="px-4 py-3 text-text-main text-right" onclick={(e) => e.stopPropagation()}>
                <Menu items={actions(row)} />
              </td>
            {/if}
          </tr>
        {/each}
        {#if sortedData.length === 0}
          <tr>
            <td
              colspan={visibleColumns.length + (showSelection ? 1 : 0) + (actions ? 1 : 0)}
              class="px-4 py-8 text-center text-text-muted"> No data available </td
            >
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if batchActions && selectedIds.size > 0}
    <div
      class="fixed bottom-6 left-1/2 -translate-x-1/2 bg-bg-popover border border-border-main shadow-xl rounded-full px-4 py-2 flex items-center gap-2 z-50 animate-in fade-in slide-in-from-bottom-4"
    >
      <span class="text-sm font-medium mr-2">{selectedIds.size} selected</span>
      <div class="h-4 w-px bg-border-subtle mx-1"></div>
      {#each batchActions as action}
        <Button
          variant="ghost"
          size="sm"
          class={action.danger ? "text-error hover:bg-error/10" : ""}
          onclick={() => {
            action.action(Array.from(selectedIds));
            selectedIds = new Set(); // Clear selection after action
          }}
        >
          {#if action.icon}
            <action.icon size={16} class="mr-2" />
          {/if}
          {action.label}
        </Button>
      {/each}
    </div>
  {/if}
</div>
