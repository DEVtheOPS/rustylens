<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import DataTable from "$lib/components/ui/DataTable.svelte";
  import Button from "$lib/components/ui/Button.svelte";
  import { clustersStore, type Cluster } from "$lib/stores/clusters.svelte";
  import { bookmarksStore } from "$lib/stores/bookmarks.svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { Plus, Settings as SettingsIcon, ExternalLink, Pin, PinOff } from "lucide-svelte";
  import type { Column } from "$lib/components/ui/DataTable.svelte";
  import type { MenuItem } from "$lib/components/ui/Menu.svelte";

  let search = $state("");
  let loading = $state(false);

  const columns: Column[] = [
    { id: "icon", label: "Icon", sortable: false },
    { id: "name", label: "Name", sortable: true },
    { id: "context_name", label: "Context", sortable: true },
    { id: "description", label: "Description", sortable: false },
    { id: "tags", label: "Tags", sortable: false },
    { id: "last_accessed", label: "Last Accessed", sortable: true },
  ];

  const filteredClusters = $derived(
    clustersStore.clusters.filter((cluster) => {
      const searchLower = search.toLowerCase();
      return (
        cluster.name.toLowerCase().includes(searchLower) ||
        cluster.context_name.toLowerCase().includes(searchLower) ||
        (cluster.description && cluster.description.toLowerCase().includes(searchLower))
      );
    })
  );

  onMount(() => {
    clustersStore.load();
  });

  async function handleRefresh() {
    loading = true;
    await clustersStore.load();
    loading = false;
  }

  function handleRowClick(cluster: Cluster) {
    goto(`/cluster/${cluster.id}`);
  }

  function getActions(cluster: Cluster): MenuItem[] {
    const isBookmarked = bookmarksStore.isBookmarked(cluster.id);
    
    return [
      {
        label: "Open",
        action: () => goto(`/cluster/${cluster.id}`),
      },
      {
        label: isBookmarked ? "Unpin from Sidebar" : "Pin to Sidebar",
        action: () => {
          bookmarksStore.toggle(cluster.id);
        },
      },
      {
        label: "Settings",
        action: () => goto(`/cluster/${cluster.id}/settings`),
      },
      {
        label: "Delete",
        action: async () => {
          const confirmed = await confirm(
            `Are you sure you want to delete cluster "${cluster.name}"? This will remove the cluster configuration.`,
            { title: "Delete Cluster", kind: "warning" }
          );

          if (confirmed) {
            try {
              await clustersStore.remove(cluster.id);
            } catch (e) {
              console.error("Failed to delete cluster", e);
            }
          }
        },
      },
    ];
  }

  function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 30) return `${diffDays}d ago`;
    return date.toLocaleDateString();
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

<div class="flex flex-col h-full w-full overflow-hidden bg-bg-panel">
  <!-- Header -->
  <div class="p-6 border-b border-border-subtle bg-bg-main">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Clusters</h1>
        <p class="text-text-muted mt-1">
          Manage your Kubernetes clusters
        </p>
      </div>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    {#if clustersStore.clusters.length === 0 && !clustersStore.loading}
      <div class="flex flex-col items-center justify-center h-full text-center space-y-4">
        <div class="text-6xl">🚀</div>
        <h2 class="text-xl font-semibold">No Clusters Yet</h2>
        <p class="text-text-muted max-w-md">
          Get started by adding your first Kubernetes cluster. Click the + button in the
          sidebar to import a kubeconfig file.
        </p>
      </div>
    {:else}
      <DataTable
        data={filteredClusters}
        {columns}
        bind:search
        {loading}
        onRefresh={handleRefresh}
        showSearch={true}
        showRefresh={true}
        storageKey="clusters-overview"
        actions={getActions}
        onRowClick={handleRowClick}
      >
        {#snippet children({ row: cluster, column })}
          {#if column.id === "icon"}
            <div class="flex items-center justify-center">
              {#if cluster.icon}
                {#if cluster.icon.startsWith("http")}
                  <img src={cluster.icon} alt={cluster.name} class="w-8 h-8 rounded" />
                {:else}
                  <span class="text-2xl">{cluster.icon}</span>
                {/if}
              {:else}
                <div
                  class="w-8 h-8 rounded flex items-center justify-center text-sm font-bold text-white"
                  style="background-color: {getColorForString(cluster.name)}"
                >
                  {cluster.name.charAt(0).toUpperCase()}
                </div>
              {/if}
            </div>
          {:else if column.id === "name"}
            <div class="flex items-center gap-2">
              <span class="font-medium">{cluster.name}</span>
              {#if bookmarksStore.isBookmarked(cluster.id)}
                <Pin size={14} class="text-primary" />
              {/if}
            </div>
          {:else if column.id === "context_name"}
            <span class="text-text-muted text-sm font-mono">{cluster.context_name}</span>
          {:else if column.id === "description"}
            <span class="text-text-muted">{cluster.description || "-"}</span>
          {:else if column.id === "tags"}
            <div class="flex flex-wrap gap-1">
              {#each clustersStore.getTags(cluster) as tag}
                <span
                  class="px-2 py-0.5 bg-bg-panel border border-border-main rounded text-xs"
                >
                  {tag}
                </span>
              {/each}
            </div>
          {:else if column.id === "last_accessed"}
            <span class="text-text-muted text-sm">
              {formatTimestamp(cluster.last_accessed)}
            </span>
          {/if}
        {/snippet}
      </DataTable>
    {/if}
  </div>
</div>
