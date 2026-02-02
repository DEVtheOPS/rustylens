<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { headerStore } from "$lib/stores/header.svelte";
  import { activeClusterStore } from "$lib/stores/activeCluster.svelte";
  import DataTable, { type Column } from "$lib/components/ui/DataTable.svelte";
  import type { MenuItem } from "$lib/components/ui/Menu.svelte";
  import { Trash2, Eye } from "lucide-svelte";
  import DeploymentDetailDrawer from "$lib/components/DeploymentDetailDrawer.svelte";

  let data = $state<any[]>([]);
  let loading = $state(false);
  let search = $state("");

  // Detail Drawer state
  let showDrawer = $state(false);
  let selectedDeployment = $state({
    name: '',
    namespace: '',
  });

  const columns: Column[] = [
    { id: "name", label: "Name", sortable: true },
    { id: "namespace", label: "Namespace", sortable: true },
    { id: "status", label: "Status", sortable: true },
    { id: "images", label: "Images", sortable: true },
    { id: "age", label: "Age", sortable: true, sortKey: "created_at" },
  ];

  $effect(() => {
    headerStore.setTitle("Deployments");
  });

  $effect(() => {
    if (activeClusterStore.clusterId) {
      loadData();
    }
  });

  async function loadData() {
    loading = true;
    try {
      data = await invoke("cluster_list_deployments", {
        clusterId: activeClusterStore.clusterId,
        namespace: activeClusterStore.activeNamespace === "all" ? null : activeClusterStore.activeNamespace,
      });
    } catch (e) {
      console.error("Failed to load deployments", e);
    } finally {
      loading = false;
    }
  }

  function handleRowClick(row: any) {
    selectedDeployment = {
      name: row.name,
      namespace: row.namespace,
    };
    showDrawer = true;
  }

  async function handleBatchDelete(selectedIds: any[]) {
    const itemsToDelete = data.filter((item) => selectedIds.includes(item.id));

    const confirmed = await confirm(
      `Are you sure you want to delete ${itemsToDelete.length} deployment(s)?`,
      { title: "Delete Deployments", kind: "warning" }
    );

    if (confirmed) {
      let successCount = 0;
      for (const item of itemsToDelete) {
        try {
          await invoke("cluster_delete_deployment", {
            clusterId: activeClusterStore.clusterId,
            namespace: item.namespace,
            name: item.name,
          });
          successCount++;
        } catch (e) {
          console.error(`Failed to delete ${item.name}`, e);
        }
      }
      if (successCount > 0) {
        loadData();
      }
    }
  }

  function getActions(row: any): MenuItem[] {
    return [
      {
        label: "View Details",
        action: () => {
          selectedDeployment = {
            name: row.name,
            namespace: row.namespace,
          };
          showDrawer = true;
        },
        icon: Eye,
      },
      {
        label: "Delete",
        action: async () => {
          const confirmed = await confirm(
            `Are you sure you want to delete ${row.name}?`,
            { title: "Delete Deployment", kind: "warning" }
          );

          if (confirmed) {
            try {
              await invoke("cluster_delete_deployment", {
                clusterId: activeClusterStore.clusterId,
                namespace: row.namespace,
                name: row.name,
              });
              loadData();
            } catch (e) {
              console.error("Failed to delete", e);
            }
          }
        },
        icon: Trash2,
        danger: true,
      },
    ];
  }
</script>

<div class="h-full">
  <DataTable
    {data}
    {columns}
    bind:search
    {loading}
    onRefresh={loadData}
    actions={getActions}
    onRowClick={handleRowClick}
    batchActions={[
      {
        label: "Delete",
        icon: Trash2,
        danger: true,
        action: handleBatchDelete
      }
    ]}
    storageKey="workload-deployments"
  >
    {#snippet children({ row, column, value })}
      {#if column.id === "images"}
        <div class="flex flex-col gap-1">
          {#if Array.isArray(value)}
            {#each value.slice(0, 2) as img}
              <span class="text-xs font-mono bg-bg-panel px-1 rounded truncate max-w-[200px]" title={img}>
                {img.split('/').pop()}
              </span>
            {/each}
            {#if value.length > 2}
              <span class="text-xs text-text-muted">+{value.length - 2} more</span>
            {/if}
          {/if}
        </div>
      {:else if column.id === "status"}
        <span class="font-medium font-mono">{value}</span>
      {:else}
        {value}
      {/if}
    {/snippet}
  </DataTable>

  <DeploymentDetailDrawer
    bind:open={showDrawer}
    bind:deploymentName={selectedDeployment.name}
    bind:namespace={selectedDeployment.namespace}
  />
</div>
