<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import Button from "$lib/components/ui/Button.svelte";
  import Input from "$lib/components/ui/Input.svelte";
  import { X, FileText, Folder, Loader2 } from "lucide-svelte";
  import { clustersStore } from "$lib/stores/clusters.svelte";

  let { isOpen = $bindable(), onClose } = $props<{
    isOpen: boolean;
    onClose: () => void;
  }>();

  let activeTab = $state<"file" | "folder">("file");
  let loading = $state(false);
  let error = $state<string | null>(null);

  interface DiscoveredContext {
    context_name: string;
    cluster_name: string;
    user_name: string;
    namespace: string | null;
    source_file: string;
    display_name: string;
    icon: string;
  }

  let discoveredContexts = $state<DiscoveredContext[]>([]);
  let selectedContexts = $state<Set<string>>(new Set());

  async function handleImportFile() {
    loading = true;
    error = null;
    discoveredContexts = [];
    selectedContexts = new Set();

    try {
      const selected = await open({
        multiple: false,
        title: "Select Kubeconfig File",
      });

      if (!selected) {
        loading = false;
        return;
      }

      const contexts = await invoke<DiscoveredContext[]>("import_discover_file", {
        path: selected,
      });

      discoveredContexts = contexts.map((ctx) => ({
        ...ctx,
        display_name: ctx.context_name,
        icon: "🌐",
      }));

      // Select all by default
      contexts.forEach((ctx) => selectedContexts.add(ctx.context_name));
    } catch (e) {
      error = `Failed to import file: ${e}`;
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function handleImportFolder() {
    loading = true;
    error = null;
    discoveredContexts = [];
    selectedContexts = new Set();

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Folder with Kubeconfig Files",
      });

      if (!selected) {
        loading = false;
        return;
      }

      const contexts = await invoke<DiscoveredContext[]>("import_discover_folder", {
        path: selected,
      });

      discoveredContexts = contexts.map((ctx) => ({
        ...ctx,
        display_name: ctx.context_name,
        icon: "🌐",
      }));

      // Select all by default
      contexts.forEach((ctx) => selectedContexts.add(ctx.context_name));
    } catch (e) {
      error = `Failed to import folder: ${e}`;
      console.error(e);
    } finally {
      loading = false;
    }
  }

  function toggleContext(contextName: string) {
    if (selectedContexts.has(contextName)) {
      selectedContexts.delete(contextName);
    } else {
      selectedContexts.add(contextName);
    }
    selectedContexts = new Set(selectedContexts); // Trigger reactivity
  }

  function updateDisplayName(contextName: string, newName: string) {
    const ctx = discoveredContexts.find((c) => c.context_name === contextName);
    if (ctx) {
      ctx.display_name = newName;
    }
  }

  function updateIcon(contextName: string, newIcon: string) {
    const ctx = discoveredContexts.find((c) => c.context_name === contextName);
    if (ctx) {
      ctx.icon = newIcon;
    }
  }

  async function handleImportSelected() {
    loading = true;
    error = null;

    try {
      const toImport = discoveredContexts.filter((ctx) =>
        selectedContexts.has(ctx.context_name)
      );

      for (const ctx of toImport) {
        await invoke("import_add_cluster", {
          name: ctx.display_name,
          context_name: ctx.context_name,
          source_file: ctx.source_file,
          icon: ctx.icon !== "🌐" ? ctx.icon : null,
          description: null,
          tags: [],
        });
      }

      await clustersStore.load();
      onClose();
    } catch (e) {
      error = `Failed to import clusters: ${e}`;
      console.error(e);
    } finally {
      loading = false;
    }
  }

  function handleClose() {
    discoveredContexts = [];
    selectedContexts = new Set();
    error = null;
    onClose();
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
    onclick={handleClose}
  >
    <!-- Modal -->
    <div
      class="bg-bg-main rounded-lg shadow-xl w-full max-w-3xl max-h-[80vh] flex flex-col"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-border-main">
        <h2 class="text-lg font-semibold">Import Clusters</h2>
        <button
          onclick={handleClose}
          class="p-1 hover:bg-bg-panel rounded transition-colors"
        >
          <X size={20} />
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-border-main">
        <button
          onclick={() => {
            activeTab = "file";
            discoveredContexts = [];
            selectedContexts = new Set();
            error = null;
          }}
          class="px-4 py-2 font-medium transition-colors"
          class:border-b-2={activeTab === "file"}
          class:border-primary={activeTab === "file"}
          class:text-primary={activeTab === "file"}
          class:text-text-muted={activeTab !== "file"}
        >
          <div class="flex items-center gap-2">
            <FileText size={16} />
            Import from File
          </div>
        </button>
        <button
          onclick={() => {
            activeTab = "folder";
            discoveredContexts = [];
            selectedContexts = new Set();
            error = null;
          }}
          class="px-4 py-2 font-medium transition-colors"
          class:border-b-2={activeTab === "folder"}
          class:border-primary={activeTab === "folder"}
          class:text-primary={activeTab === "folder"}
          class:text-text-muted={activeTab !== "folder"}
        >
          <div class="flex items-center gap-2">
            <Folder size={16} />
            Import from Folder
          </div>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-4">
        {#if activeTab === "file"}
          <div class="space-y-4">
            <div class="bg-bg-panel p-4 rounded-lg border border-border-main">
              <div class="flex items-start gap-3">
                <div class="flex-shrink-0 w-6 h-6 rounded-full bg-primary/20 text-primary flex items-center justify-center text-sm font-bold">
                  1
                </div>
                <div class="flex-1">
                  <p class="font-medium mb-1">Select a kubeconfig file</p>
                  <p class="text-text-muted text-sm">
                    Choose a YAML file containing your Kubernetes cluster configuration. 
                    If the file has multiple contexts, you'll be able to select which ones to import.
                  </p>
                </div>
              </div>
            </div>

            {#if discoveredContexts.length === 0}
              <div class="flex justify-center">
                <Button onclick={handleImportFile} disabled={loading}>
                  {#if loading}
                    <Loader2 size={16} class="animate-spin" />
                    Analyzing file...
                  {:else}
                    <FileText size={16} />
                    Choose Kubeconfig File
                  {/if}
                </Button>
              </div>
            {/if}
          </div>
        {:else}
          <div class="space-y-4">
            <div class="bg-bg-panel p-4 rounded-lg border border-border-main">
              <div class="flex items-start gap-3">
                <div class="flex-shrink-0 w-6 h-6 rounded-full bg-primary/20 text-primary flex items-center justify-center text-sm font-bold">
                  1
                </div>
                <div class="flex-1">
                  <p class="font-medium mb-1">Select a folder</p>
                  <p class="text-text-muted text-sm">
                    Choose a folder to scan for kubeconfig files. All valid configurations 
                    and their contexts will be discovered automatically.
                  </p>
                </div>
              </div>
            </div>

            {#if discoveredContexts.length === 0}
              <div class="flex justify-center">
                <Button onclick={handleImportFolder} disabled={loading}>
                  {#if loading}
                    <Loader2 size={16} class="animate-spin" />
                    Scanning folder...
                  {:else}
                    <Folder size={16} />
                    Choose Folder
                  {/if}
                </Button>
              </div>
            {/if}
          </div>
        {/if}

        {#if error}
          <div class="mt-4 p-3 bg-red-500/10 border border-red-500/20 rounded text-red-400 text-sm">
            {error}
          </div>
        {/if}

        {#if discoveredContexts.length > 0}
          <div class="mt-6 space-y-3">
            <div class="flex items-start gap-3 mb-4">
              <div class="flex-shrink-0 w-6 h-6 rounded-full bg-primary/20 text-primary flex items-center justify-center text-sm font-bold">
                2
              </div>
              <div class="flex-1">
                <h3 class="font-semibold">
                  Configure and Import ({selectedContexts.size} of {discoveredContexts.length} selected)
                </h3>
                <p class="text-text-muted text-sm mt-1">
                  Review the discovered contexts, customize their names and icons, then click Import to add them to Kore.
                </p>
              </div>
            </div>

            {#each discoveredContexts as ctx (ctx.context_name)}
              <div
                class="p-3 border rounded transition-colors"
                class:border-primary={selectedContexts.has(ctx.context_name)}
                class:border-border-main={!selectedContexts.has(ctx.context_name)}
                style={selectedContexts.has(ctx.context_name) ? "background-color: hsl(var(--primary) / 0.05)" : ""}
              >
                <div class="flex items-start gap-3">
                  <input
                    type="checkbox"
                    checked={selectedContexts.has(ctx.context_name)}
                    onchange={() => toggleContext(ctx.context_name)}
                    class="mt-1"
                  />

                  <div class="flex-1 space-y-2">
                    <div class="flex items-center gap-2">
                      <div class="flex flex-col gap-1">
                        <label class="text-xs text-text-muted">Icon</label>
                        <Input
                          value={ctx.icon}
                          oninput={(e) => updateIcon(ctx.context_name, (e.currentTarget as HTMLInputElement).value)}
                          placeholder="🌐"
                          class="w-16 text-center"
                        />
                      </div>
                      <div class="flex flex-col gap-1 flex-1">
                        <label class="text-xs text-text-muted">Display Name</label>
                        <Input
                          value={ctx.display_name}
                          oninput={(e) => updateDisplayName(ctx.context_name, (e.currentTarget as HTMLInputElement).value)}
                          placeholder="My Cluster"
                          class="flex-1"
                        />
                      </div>
                    </div>

                    <div class="text-xs text-text-muted bg-bg-main p-2 rounded">
                      <div><span class="font-medium">Context:</span> {ctx.context_name}</div>
                      <div><span class="font-medium">Cluster:</span> {ctx.cluster_name}</div>
                      <div class="truncate"><span class="font-medium">Source:</span> {ctx.source_file}</div>
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      {#if discoveredContexts.length > 0}
        <div class="flex items-center justify-end gap-2 p-4 border-t border-border-main">
          <Button variant="outline" onclick={handleClose}>Cancel</Button>
          <Button
            onclick={handleImportSelected}
            disabled={loading || selectedContexts.size === 0}
          >
            {#if loading}
              <Loader2 size={16} class="animate-spin" />
              Importing...
            {:else}
              Import {selectedContexts.size} Cluster{selectedContexts.size !== 1 ? "s" : ""}
            {/if}
          </Button>
        </div>
      {/if}
    </div>
  </div>
{/if}
