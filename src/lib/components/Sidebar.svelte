<script lang="ts">
  import {
    Layers,
    Settings,
    Box,
    FileText,
    HardDrive,
    Network,
    Anchor,
    Database,
    Cpu,
    Activity,
    Shield,
  } from "lucide-svelte";
  import { clusterStore } from "$lib/stores/cluster.svelte";
  import Select from "$lib/components/ui/Select.svelte";
  import SidebarGroup from "$lib/components/ui/SidebarGroup.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  let groups = $state({
    workloads: true,
    config: true,
    network: false,
    storage: false,
    access: false,
    helm: false,
    custom: false,
  });

  let unlisten: (() => void) | null = null;
  let refreshTimer: any;

  onMount(async () => {
    unlisten = await listen("kubeconfig_update", () => {
      console.log("Kubeconfig update detected, refreshing clusters...");
      // Frontend debounce/throttle
      if (refreshTimer) clearTimeout(refreshTimer);
      refreshTimer = setTimeout(() => {
        clusterStore.refresh();
      }, 500);
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (refreshTimer) clearTimeout(refreshTimer);
  });
</script>

<aside class="flex flex-col h-full w-full bg-bg-sidebar border-r border-border-main text-text-main">
  <!-- Cluster Switcher Area -->
  <div class="p-4 border-b border-border-subtle space-y-4">
    <div class="flex items-center gap-2 px-1">
      <img src="/rustylens.svg" alt="Rustylens" class="w-6 h-6" />
      <span class="font-bold text-lg">RustyLens</span>
    </div>

    <div class="space-y-3">
      <!-- Cluster Dropdown -->
      <div>
        <label for="cluster-select" class="text-xs font-semibold text-text-muted px-1 uppercase mb-1 block"
          >Cluster</label
        >
        <Select
          id="cluster-select"
          options={clusterStore.list}
          bind:value={clusterStore.active}
          onselect={(val) => clusterStore.setActive(val)}
          placeholder="Select Cluster"
        />
      </div>

      <!-- Namespace Dropdown -->
      <div>
        <label for="namespace-select" class="text-xs font-semibold text-text-muted px-1 uppercase mb-1 block"
          >Namespace</label
        >
        <Select
          id="namespace-select"
          options={["all", ...clusterStore.namespaces]}
          bind:value={clusterStore.activeNamespace}
          onselect={(val) => clusterStore.setNamespace(val)}
          placeholder="Namespace"
        />
      </div>
    </div>
  </div>

  <!-- Navigation Links -->
  <nav class="flex-1 overflow-y-auto py-4 px-2 space-y-1">
    <a href="/" class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-bg-popover text-sm group">
      <Cpu size={18} class="group-hover:text-primary transition-colors" />
      <span>Nodes</span>
    </a>

    <SidebarGroup title="Workloads" icon={Box} bind:open={groups.workloads}>
      <a href="/workloads" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Overview</a>
      <a href="/pods" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Pods</a>
      <a href="/deployments" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Deployments</a>
      <a href="/daemonsets" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">DaemonSets</a>
      <a href="/statefulsets" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">StatefulSets</a>
      <a href="/replicasets" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">ReplicaSets</a>
      <a href="/jobs" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Jobs</a>
      <a href="/cronjobs" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">CronJobs</a>
    </SidebarGroup>

    <SidebarGroup title="Config" icon={FileText} bind:open={groups.config}>
      <a href="/config-maps" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">ConfigMaps</a>
      <a href="/secrets" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Secrets</a>
      <a href="/resource-quotas" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Resource Quotas</a>
      <a href="/limit-ranges" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Limit Ranges</a>
      <a href="/hpa" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">HPA</a>
      <a href="/pdb" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Pod Disruption Budgets</a>
    </SidebarGroup>

    <SidebarGroup title="Network" icon={Network} bind:open={groups.network}>
      <a href="/services" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Services</a>
      <a href="/endpoints" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Endpoints</a>
      <a href="/ingresses" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Ingresses</a>
      <a href="/network-policies" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Network Policies</a>
    </SidebarGroup>

    <SidebarGroup title="Storage" icon={HardDrive} bind:open={groups.storage}>
      <a href="/pvc" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Persistent Volume Claims</a>
      <a href="/pv" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Persistent Volumes</a>
      <a href="/storage-classes" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Storage Classes</a>
    </SidebarGroup>

    <a href="/namespaces" class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-bg-popover text-sm group">
      <Layers size={18} class="group-hover:text-primary transition-colors" />
      <span>Namespaces</span>
    </a>

    <a href="/events" class="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-bg-popover text-sm group">
      <Activity size={18} class="group-hover:text-primary transition-colors" />
      <span>Events</span>
    </a>

    <SidebarGroup title="Helm" icon={Anchor} bind:open={groups.helm}>
      <a href="/helm/charts" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Charts</a>
      <a href="/helm/releases" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Releases</a>
    </SidebarGroup>

    <SidebarGroup title="Access Control" icon={Shield} bind:open={groups.access}>
      <a href="/service-accounts" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Service Accounts</a>
      <a href="/cluster-roles" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Cluster Roles</a>
      <a href="/roles" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Roles</a>
      <a href="/cluster-role-bindings" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm"
        >Cluster Role Bindings</a
      >
      <a href="/role-bindings" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Role Bindings</a>
    </SidebarGroup>

    <SidebarGroup title="Custom Resources" icon={Database} bind:open={groups.custom}>
      <a href="/crd" class="block px-3 py-1.5 rounded-md hover:bg-bg-popover text-sm">Definitions</a>
    </SidebarGroup>
  </nav>

  <!-- Bottom Settings -->
  <div class="p-4 border-t border-border-subtle">
    <a
      href="/settings/clusters"
      class="flex items-center gap-3 px-3 py-2 w-full rounded-md hover:bg-bg-popover text-sm"
    >
      <Settings size={18} />
      <span>Settings</span>
    </a>
  </div>
</aside>
