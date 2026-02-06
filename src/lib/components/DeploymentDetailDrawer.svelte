<script lang="ts">
  import Drawer from '$lib/components/ui/Drawer.svelte';
  import Badge from '$lib/components/ui/Badge.svelte';
  import Chart from '$lib/components/ui/Chart.svelte';
  import { Edit, RefreshCw, Trash2 } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { activeClusterStore } from '$lib/stores/activeCluster.svelte';
  import yaml from 'js-yaml';

  interface DeploymentCondition {
    condition_type: string;
    status: string;
    reason?: string;
    message?: string;
    last_transition_time?: string;
  }

  interface DeploymentDetails {
    name: string;
    namespace: string;
    uid: string;
    created_at: string;
    labels: Record<string, string>;
    annotations: Record<string, string>;
    replicas_desired: number;
    replicas_updated: number;
    replicas_total: number;
    replicas_available: number;
    replicas_unavailable: number;
    strategy_type: string;
    selector: Record<string, string>;
    conditions: DeploymentCondition[];
    images: string[];
  }

  interface DeploymentPodInfo {
    name: string;
    namespace: string;
    status: string;
    age: string;
    ready: string;
    restarts: number;
    node: string;
    pod_ip: string;
  }

  interface ReplicaSetInfo {
    name: string;
    namespace: string;
    revision: string;
    desired: number;
    current: number;
    ready: number;
    age: string;
    images: string[];
    created_at: string;
  }

  interface K8sEventInfo {
    event_type: string;
    reason: string;
    message: string;
    count: number;
    first_timestamp?: string;
    last_timestamp?: string;
    source: string;
  }

  let {
    open = $bindable(false),
    deploymentName = $bindable(''),
    namespace = $bindable(''),
  }: {
    open: boolean;
    deploymentName: string;
    namespace: string;
  } = $props();

  let details = $state<DeploymentDetails | null>(null);
  let pods = $state<DeploymentPodInfo[]>([]);
  let replicaSets = $state<ReplicaSetInfo[]>([]);
  let events = $state<K8sEventInfo[]>([]);
  let loading = $state(false);
  let activeTab = $state<'cpu' | 'memory' | 'network' | 'filesystem'>('cpu');

  // Fetch deployment details when drawer opens
  $effect(() => {
    if (open && deploymentName && namespace) {
      loadDeploymentDetails();
    }
  });

  async function loadDeploymentDetails() {
    if (!activeClusterStore.clusterId) return;

    loading = true;
    try {
      // Fetch all data in parallel
      const [detailsData, podsData, replicaSetsData, eventsData] = await Promise.all([
        invoke<DeploymentDetails>('cluster_get_deployment_details', {
          clusterId: activeClusterStore.clusterId,
          namespace,
          name: deploymentName,
        }),
        invoke<DeploymentPodInfo[]>('cluster_get_deployment_pods', {
          clusterId: activeClusterStore.clusterId,
          namespace,
          deploymentName,
        }),
        invoke<ReplicaSetInfo[]>('cluster_get_deployment_replicasets', {
          clusterId: activeClusterStore.clusterId,
          namespace,
          deploymentName,
        }),
        invoke<K8sEventInfo[]>('cluster_get_deployment_events', {
          clusterId: activeClusterStore.clusterId,
          namespace,
          deploymentName,
        }),
      ]);

      details = detailsData;
      pods = podsData;
      replicaSets = replicaSetsData;
      events = eventsData;
    } catch (error) {
      console.error('Failed to load deployment details:', error);
    } finally {
      loading = false;
    }
  }

  function handleRefresh() {
    loadDeploymentDetails();
  }

  function handleEdit() {
    // TODO: Implement edit functionality
    console.log('Edit deployment:', deploymentName);
  }

  function handleDelete() {
    // TODO: Implement delete functionality
    console.log('Delete deployment:', deploymentName);
  }

  function getConditionVariant(status: string): 'success' | 'warning' | 'error' | 'info' | 'neutral' {
    if (status === 'True') return 'success';
    if (status === 'False') return 'error';
    return 'neutral';
  }

  function formatAnnotationValue(value: string): { formatted: string; isYaml: boolean } {
    try {
      // Try to parse as JSON
      const parsed = JSON.parse(value);
      // Convert to YAML
      const yamlStr = yaml.dump(parsed, { indent: 2, lineWidth: -1 });
      return { formatted: yamlStr, isYaml: true };
    } catch {
      // Not JSON, return as-is
      return { formatted: value, isYaml: false };
    }
  }

  function handlePodClick(pod: DeploymentPodInfo) {
    // Get the cluster ID from the current page params
    const clusterId = $page.params.id;
    // Navigate to pods page with query params to auto-open the pod
    goto(`/cluster/${clusterId}/pods?pod=${encodeURIComponent(pod.name)}&namespace=${encodeURIComponent(pod.namespace)}`);
  }

  // Mock chart data for now
  const chartData = {
    labels: ['09:32', '09:42', '09:52', '10:02', '10:12', '10:22', '10:32'],
    datasets: [
      {
        label: 'Usage',
        data: [0.000, 0.000, 0.000, 0.001, 0.000, 0.000, 0.000],
        borderColor: 'rgb(75, 192, 192)',
        tension: 0.1,
      },
    ],
  };

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      y: {
        beginAtZero: true,
      },
    },
  };
</script>

<Drawer bind:open title="Deployment: {deploymentName}" width="w-[800px]">
  {#snippet headerActions()}
    <button
      class="p-1.5 hover:bg-bg-panel rounded-md text-text-muted hover:text-text-main transition-colors"
      onclick={handleRefresh}
      title="Refresh"
    >
      <RefreshCw size={18} />
    </button>
    <button
      class="p-1.5 hover:bg-bg-panel rounded-md text-text-muted hover:text-text-main transition-colors"
      onclick={handleEdit}
      title="Edit"
    >
      <Edit size={18} />
    </button>
    <button
      class="p-1.5 hover:bg-bg-panel rounded-md text-text-muted hover:text-text-main transition-colors"
      onclick={handleDelete}
      title="Delete"
    >
      <Trash2 size={18} />
    </button>
  {/snippet}

  {#if loading}
    <div class="flex items-center justify-center py-8">
      <div class="text-text-muted">Loading deployment details...</div>
    </div>
  {:else if details}
    <div class="space-y-6">
      <!-- Metrics Tabs -->
      <div class="space-y-4">
        <div class="flex gap-2 border-b border-border">
          <button
            class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'cpu'
              ? 'border-b-2 border-color-primary text-text-main'
              : 'text-text-muted hover:text-text-main'}"
            onclick={() => (activeTab = 'cpu')}
          >
            CPU
          </button>
          <button
            class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'memory'
              ? 'border-b-2 border-color-primary text-text-main'
              : 'text-text-muted hover:text-text-main'}"
            onclick={() => (activeTab = 'memory')}
          >
            Memory
          </button>
          <button
            class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'network'
              ? 'border-b-2 border-color-primary text-text-main'
              : 'text-text-muted hover:text-text-main'}"
            onclick={() => (activeTab = 'network')}
          >
            Network
          </button>
          <button
            class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'filesystem'
              ? 'border-b-2 border-color-primary text-text-main'
              : 'text-text-muted hover:text-text-main'}"
            onclick={() => (activeTab = 'filesystem')}
          >
            Filesystem
          </button>
        </div>

        <!-- Chart -->
        <div class="h-48 bg-bg-panel rounded-md p-4">
          <Chart type="line" data={chartData} options={chartOptions} />
        </div>
      </div>

      <!-- Deployment Details -->
      <div class="space-y-4">
        <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
          Details
        </h3>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Created</div>
            <div class="text-sm">{new Date(details.created_at).toLocaleString()}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Name</div>
            <div class="text-sm font-mono">{details.name}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Namespace</div>
            <div class="text-sm">{details.namespace}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Replicas</div>
            <div class="text-sm">
              {details.replicas_desired} desired, {details.replicas_updated} updated,
              {details.replicas_total} total, {details.replicas_available} available,
              {details.replicas_unavailable} unavailable
            </div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Strategy Type</div>
            <div class="text-sm">{details.strategy_type}</div>
          </div>
        </div>
      </div>

      <!-- Labels -->
      {#if Object.keys(details.labels).length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Labels ({Object.keys(details.labels).length})
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each Object.entries(details.labels) as [key, value]}
              <Badge variant="neutral">
                <span class="font-mono text-xs">{key}={value}</span>
              </Badge>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Annotations -->
      {#if Object.keys(details.annotations).length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Annotations ({Object.keys(details.annotations).length})
          </h3>
          <div class="space-y-2 max-h-[500px] overflow-y-auto">
            {#each Object.entries(details.annotations) as [key, value]}
              {@const { formatted, isYaml } = formatAnnotationValue(value)}
              <div class="p-3 bg-bg-panel rounded-md">
                <div class="text-text-muted font-semibold mb-2 text-xs">{key}</div>
                {#if isYaml}
                  <pre class="text-xs overflow-x-auto bg-bg-main rounded p-2"><code class="language-yaml">{formatted}</code></pre>
                {:else}
                  <div class="text-xs font-mono break-all text-text">{formatted}</div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Selector -->
      {#if Object.keys(details.selector).length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Selector
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each Object.entries(details.selector) as [key, value]}
              <Badge variant="info">
                <span class="font-mono text-xs">{key}={value}</span>
              </Badge>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Conditions -->
      {#if details.conditions.length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Conditions
          </h3>
          <div class="flex flex-wrap gap-2">
            {#each details.conditions as condition}
              <Badge variant={getConditionVariant(condition.status)}>
                {condition.condition_type}: {condition.status}
              </Badge>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Deploy Revisions -->
      {#if replicaSets.length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Deploy Revisions
          </h3>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead class="text-xs text-text-muted uppercase border-b border-border">
                <tr>
                  <th class="text-left py-2 px-3">Name</th>
                  <th class="text-left py-2 px-3">Revision</th>
                  <th class="text-left py-2 px-3">Pods</th>
                  <th class="text-left py-2 px-3">Age</th>
                </tr>
              </thead>
              <tbody>
                {#each replicaSets as rs}
                  <tr class="border-b border-border/50 hover:bg-bg-panel/50">
                    <td class="py-2 px-3 font-mono text-xs">{rs.name}</td>
                    <td class="py-2 px-3">{rs.revision}</td>
                    <td class="py-2 px-3">{rs.ready}/{rs.desired}</td>
                    <td class="py-2 px-3">{rs.age}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- Pods -->
      {#if pods.length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Pods ({pods.length})
          </h3>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead class="text-xs text-text-muted uppercase border-b border-border">
                <tr>
                  <th class="text-left py-2 px-3">Name</th>
                  <th class="text-left py-2 px-3">Ready</th>
                  <th class="text-left py-2 px-3">Status</th>
                  <th class="text-left py-2 px-3">Restarts</th>
                  <th class="text-left py-2 px-3">Age</th>
                </tr>
              </thead>
              <tbody>
                {#each pods as pod}
                  <tr
                    class="border-b border-border/50 hover:bg-bg-panel/50 cursor-pointer transition-colors"
                    onclick={() => handlePodClick(pod)}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) => e.key === 'Enter' && handlePodClick(pod)}
                  >
                    <td class="py-2 px-3 font-mono text-xs">{pod.name}</td>
                    <td class="py-2 px-3">{pod.ready}</td>
                    <td class="py-2 px-3">
                      <Badge variant={pod.status === 'Running' ? 'success' : 'warning'}>
                        {pod.status}
                      </Badge>
                    </td>
                    <td class="py-2 px-3">{pod.restarts}</td>
                    <td class="py-2 px-3">{pod.age}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- Events -->
      <div class="space-y-4">
        <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
          Events {#if events.length > 0}({events.length}){/if}
        </h3>
        {#if events.length > 0}
          <div class="space-y-2 max-h-96 overflow-y-auto">
            {#each events as event}
              <div class="p-3 bg-bg-panel rounded-md">
                <div class="flex items-start justify-between gap-2 mb-2">
                  <div class="flex items-center gap-2">
                    <Badge variant={event.event_type === 'Warning' ? 'error' : 'neutral'}>
                      {event.event_type}
                    </Badge>
                    <span class="text-sm font-semibold">{event.reason}</span>
                  </div>
                  {#if event.count > 1}
                    <Badge variant="neutral">{event.count}x</Badge>
                  {/if}
                </div>
                <div class="text-xs text-text mb-2">{event.message}</div>
                <div class="flex items-center justify-between text-xs text-text-muted">
                  <div>Source: {event.source}</div>
                  {#if event.last_timestamp}
                    <div>{new Date(event.last_timestamp).toLocaleString()}</div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-sm text-text-muted text-center py-4">No events found</div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="text-sm text-text-muted text-center py-8">No deployment details available</div>
  {/if}
</Drawer>
