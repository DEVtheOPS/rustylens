<script lang="ts">
  import { Home, Plus, Settings as SettingsIcon, MoreVertical } from "lucide-svelte";
  import { clustersStore } from "$lib/stores/clusters.svelte";
  import { bookmarksStore } from "$lib/stores/bookmarks.svelte";
  import { page } from "$app/stores";
  import Menu from "$lib/components/ui/Menu.svelte";
  import type { MenuItem } from "$lib/components/ui/Menu.svelte";

  let { onAddCluster } = $props<{
    onAddCluster: () => void;
  }>();

  let draggedIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  const bookmarkedClusterIds = $derived(bookmarksStore.getBookmarkedClusterIds());
  const bookmarkedClusters = $derived(
    bookmarkedClusterIds
      .map((id) => clustersStore.clusters.find((c) => c.id === id))
      .filter((c) => c !== undefined)
  );

  function handleDragStart(e: DragEvent, index: number) {
    draggedIndex = index;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
    }
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    dragOverIndex = index;
  }

  function handleDragEnd() {
    if (draggedIndex !== null && dragOverIndex !== null && draggedIndex !== dragOverIndex) {
      bookmarksStore.reorder(draggedIndex, dragOverIndex);
    }
    draggedIndex = null;
    dragOverIndex = null;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    handleDragEnd();
  }

  function getClusterMenuItems(clusterId: string): MenuItem[] {
    return [
      {
        label: bookmarksStore.isBookmarked(clusterId) ? "Remove from Bookmarks" : "Add to Bookmarks",
        action: () => bookmarksStore.toggle(clusterId),
      },
      {
        label: "Open",
        action: () => {
          window.location.href = `/cluster/${clusterId}`;
        },
      },
      {
        label: "Settings",
        action: () => {
          window.location.href = `/cluster/${clusterId}/settings`;
        },
      },
    ];
  }

  function isActive(path: string): boolean {
    return $page.url.pathname === path || $page.url.pathname.startsWith(path + "/");
  }

  function getColorForString(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const hue = hash % 360;
    return `hsl(${hue}, 60%, 50%)`;
  }
</script>

<aside class="flex flex-col h-full w-16 bg-bg-sidebar border-r border-border-main">
  <!-- Overview / Home -->
  <a
    href="/"
    class="flex items-center justify-center h-16 hover:bg-bg-main transition-colors relative group"
    class:bg-bg-main={isActive("/")}
    title="Overview"
  >
    <Home size={28} class={isActive("/") && !$page.url.pathname.startsWith("/cluster") ? "text-primary" : "text-text-main"} />
  </a>

  <!-- Add Cluster -->
  <button
    onclick={onAddCluster}
    class="flex items-center justify-center h-16 hover:bg-bg-main transition-colors"
    title="Add Cluster"
  >
    <Plus size={28} />
  </button>

  <!-- Divider -->
  <div class="h-px bg-border-subtle mx-2 my-1"></div>

  <!-- Bookmarked Clusters -->
  <div class="flex-1 overflow-y-auto">
    {#each bookmarkedClusters as cluster, index (cluster?.id)}
      {#if cluster}
        <div
          class="relative group"
          draggable="true"
          ondragstart={(e) => handleDragStart(e, index)}
          ondragover={(e) => handleDragOver(e, index)}
          ondragend={handleDragEnd}
          ondrop={handleDrop}
          role="listitem"
        >
          <!-- Drag indicator -->
          {#if dragOverIndex === index && draggedIndex !== index}
            <div class="absolute top-0 left-0 right-0 h-0.5 bg-primary"></div>
          {/if}

          <a
            href="/cluster/{cluster.id}"
            class="flex items-center justify-center h-16 hover:bg-bg-main transition-colors relative"
            class:bg-bg-main={isActive(`/cluster/${cluster.id}`)}
            title={cluster.name}
          >
            {#if cluster.icon}
              {#if cluster.icon.startsWith("http") || cluster.icon.startsWith("data:")}
                <img src={cluster.icon} alt={cluster.name} class="w-10 h-10 rounded object-contain" />
              {:else}
                <span class="text-3xl">{cluster.icon}</span>
              {/if}
            {:else}
              <div
                class="w-10 h-10 rounded flex items-center justify-center text-sm font-bold"
                style="background-color: {getColorForString(cluster.name)}; color: white;"
              >
                {cluster.name.charAt(0).toUpperCase()}
              </div>
            {/if}

            <!-- Active indicator -->
            {#if isActive(`/cluster/${cluster.id}`)}
              <div class="absolute left-0 top-2 bottom-2 w-0.5 bg-primary"></div>
            {/if}
          </a>

          <!-- Context menu trigger -->
          <div class="absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <Menu items={getClusterMenuItems(cluster.id)} align="left" />
          </div>
        </div>
      {/if}
    {/each}
  </div>

  <!-- Divider -->
  <div class="h-px bg-border-subtle mx-2 my-1"></div>

  <!-- Settings -->
  <a
    href="/settings"
    class="flex items-center justify-center h-16 hover:bg-bg-main transition-colors relative"
    class:bg-bg-main={isActive("/settings")}
    title="Settings"
  >
    <SettingsIcon size={28} class={isActive("/settings") ? "text-primary" : "text-text-main"} />
  </a>
</aside>

<style>
  /* Custom scrollbar for bookmarks */
  .overflow-y-auto::-webkit-scrollbar {
    width: 4px;
  }

  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }

  .overflow-y-auto::-webkit-scrollbar-thumb {
    background: var(--border-main);
    border-radius: 2px;
  }

  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }
</style>
