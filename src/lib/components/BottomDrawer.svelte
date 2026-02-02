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
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem("bottom-drawer-height");
      if (saved) {
        drawerHeight = parseInt(saved, 10);
      }
    }
  });

  function startResize(e: MouseEvent) {
    // If drawer is closed, open it and set height based on mouse position
    if (!bottomDrawerStore.open) {
      bottomDrawerStore.open = true;
      // Calculate initial height from bottom of viewport to mouse position
      const initialHeight = Math.max(200, Math.min(window.innerHeight - 100, window.innerHeight - e.clientY));
      drawerHeight = initialHeight;
      startHeight = initialHeight;

      // Save the new height
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("bottom-drawer-height", initialHeight.toString());
      }
    } else {
      startHeight = drawerHeight;
    }

    isResizing = true;
    startY = e.clientY;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isResizing) return;
    const deltaY = startY - e.clientY;
    const newHeight = Math.max(200, Math.min(window.innerHeight - 100, startHeight + deltaY));
    drawerHeight = newHeight;

    // Save to localStorage as user drags
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("bottom-drawer-height", newHeight.toString());
    }
  }

  function handleMouseUp() {
    isResizing = false;
  }

  function handleTabBarClick(e?: MouseEvent | KeyboardEvent) {
    // Don't toggle if clicking on a tab or the minimize button
    if (e?.target) {
      const target = e.target as HTMLElement;
      if (target.closest('[role="tab"]') || target.closest("button")) {
        return;
      }
    }

    bottomDrawerStore.toggle();
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
  <!-- Resize Handle (Always Shown) -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="h-2 bg-bg-sidebar transition-colors flex items-center justify-center group"
    style="cursor: ns-resize !important;"
    onmousedown={startResize}
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize drawer"
  >
    <hr class="hover:bg-primary" />
  </div>

  <!-- Tabs Bar (Always Shown) -->
  <div
    class="flex items-center justify-between bg-bg-sidebar px-2 cursor-pointer hover:bg-bg-main/50 transition-colors"
    onclick={handleTabBarClick}
    onkeydown={(e) => e.key === 'Enter' && handleTabBarClick(e)}
    role="button"
    tabindex="0"
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
            onkeydown={(e) => e.key === 'Enter' && bottomDrawerStore.setActiveTab(tab.id)}
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
      {#if bottomDrawerStore.open}
        <ChevronDown size={18} />
      {:else}
        <ChevronUp size={18} />
      {/if}
    </div>
  </div>

  <!-- Drawer Content (Shown when open) -->
  {#if bottomDrawerStore.open}
    <div class="bg-bg-main flex flex-col" style="height: {drawerHeight}px;">
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
        {:else}
          <div class="flex items-center justify-center h-full text-text-muted">Open a pod's logs to get started</div>
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
