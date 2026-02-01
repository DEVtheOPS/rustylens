<script lang="ts">
  import { X, ChevronDown, ChevronUp } from "lucide-svelte";
  import { bottomDrawerStore } from "$lib/stores/bottomDrawer.svelte";
  import LogsTab from "./tabs/LogsTab.svelte";

  let drawerHeight = $state(400);
  let isResizing = $state(false);
  let startY = $state(0);
  let startHeight = $state(0);

  // Load saved height on mount
  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('bottom-drawer-height');
      if (saved) {
        drawerHeight = parseInt(saved, 10);
      }
    }
  });

  function startResize(e: MouseEvent) {
    isResizing = true;
    startY = e.clientY;
    startHeight = drawerHeight;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isResizing) return;
    const deltaY = startY - e.clientY;
    const newHeight = Math.max(200, Math.min(window.innerHeight - 100, startHeight + deltaY));
    drawerHeight = newHeight;
    
    // Save to localStorage as user drags
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('bottom-drawer-height', newHeight.toString());
    }
  }

  function handleMouseUp() {
    isResizing = false;
  }

  function handleTabBarClick(e: MouseEvent) {
    // Don't toggle if clicking on a tab or the minimize button
    const target = e.target as HTMLElement;
    if (target.closest('[role="tab"]') || target.closest('button')) {
      return;
    }
    
    if (bottomDrawerStore.tabs.length > 0) {
      bottomDrawerStore.toggle();
    }
  }

  $effect(() => {
    if (isResizing) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
      return () => {
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };
    }
  });
</script>

<div class="flex flex-col bg-bg-main">
  <!-- Tabs Bar (Always Shown) -->
  <div 
    class="flex items-center justify-between bg-bg-panel px-2 {bottomDrawerStore.tabs.length > 0 ? 'cursor-pointer hover:bg-bg-main/50' : ''} transition-colors"
    onclick={handleTabBarClick}
  >
    <div class="flex items-center gap-1 bg-bg-sidebar overflow-x-auto flex-1">
      {#if bottomDrawerStore.tabs.length === 0}
        <div class="px-3 py-2 text-sm text-text-muted">No tabs open</div>
      {:else}
        {#each bottomDrawerStore.tabs as tab}
          <div
            class="px-3 py-2 text-sm flex items-center gap-2 hover:bg-bg-main transition-colors cursor-pointer {bottomDrawerStore.activeTabId ===
            tab.id
              ? 'bg-bg-main'
              : ''}"
            onclick={() => bottomDrawerStore.setActiveTab(tab.id)}
            role="tab"
            tabindex="0"
          >
            <span>{tab.title}</span>
            <button
              class="hover:bg-error/20 rounded p-0.5"
              onclick={(e) => {
                e.stopPropagation();
                bottomDrawerStore.closeTab(tab.id);
              }}
            >
              <X size={14} />
            </button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="flex items-center gap-2 ml-2">
      <button
        class="p-1 hover:bg-bg-main rounded transition-colors"
        onclick={() => bottomDrawerStore.toggle()}
        title={bottomDrawerStore.open ? "Minimize" : "Maximize"}
        disabled={bottomDrawerStore.tabs.length === 0}
      >
        {#if bottomDrawerStore.open}
          <ChevronDown size={18} />
        {:else}
          <ChevronUp size={18} />
        {/if}
      </button>
    </div>
  </div>

  <!-- Drawer Content (Shown when open) -->
  {#if bottomDrawerStore.open}
    <div class="bg-bg-main flex flex-col" style="height: {drawerHeight}px;">
      <!-- Resize Handle -->
      <div
        class="h-1 bg-bg-panel hover:bg-primary cursor-ns-resize transition-colors"
        onmousedown={startResize}
        role="separator"
        aria-orientation="horizontal"
      ></div>

      <!-- Tab Content -->
      <div class="flex-1 overflow-hidden">
        {#if bottomDrawerStore.activeTab}
          {#if bottomDrawerStore.activeTab.type === "logs"}
            <LogsTab data={bottomDrawerStore.activeTab.data} />
          {:else if bottomDrawerStore.activeTab.type === "edit"}
            <div class="p-4">Edit functionality coming soon...</div>
          {:else}
            <div class="p-4">Unknown tab type</div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  button {
    user-select: none;
  }
</style>
