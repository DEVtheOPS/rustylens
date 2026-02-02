<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import { fade, fly } from 'svelte/transition';

  interface Props {
    open: boolean;
    title?: string;
    onclose?: () => void;
    children: Snippet;
    headerActions?: Snippet;
    width?: string;
  }

  let { 
    open = $bindable(false), 
    title = 'Details', 
    onclose, 
    children,
    headerActions,
    width = 'w-[600px]'
  }: Props = $props();

  function close() {
    open = false;
    onclose?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/20 z-40"
    transition:fade={{ duration: 200 }}
    onclick={close}
    onkeydown={(e) => e.key === 'Escape' && close()}
    role="button"
    tabindex="-1"
  ></div>

  <!-- Panel (Right Side standard for details) -->
  <div
    class="fixed top-0 right-0 h-full bg-bg-card border-l border-border-main shadow-xl z-50 flex flex-col {width}"
    transition:fly={{ x: 400, duration: 300 }}
    role="dialog"
    aria-modal="true"
    aria-label={title}
  >
    <div class="flex items-center justify-between p-4 border-b border-border-subtle">
      <h2 class="text-lg font-bold">{title}</h2>
      <div class="flex items-center gap-2">
        {#if headerActions}
          {@render headerActions()}
        {/if}
        <button 
          onclick={close}
          class="p-1 hover:bg-bg-panel rounded-md text-text-muted hover:text-text-main transition-colors"
        >
          <X size={20} />
        </button>
      </div>
    </div>
    
    <div class="flex-1 overflow-y-auto p-4">
      {@render children()}
    </div>
  </div>
{/if}
