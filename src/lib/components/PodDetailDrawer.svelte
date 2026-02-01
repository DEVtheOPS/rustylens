<script lang="ts">
  import Drawer from '$lib/components/ui/Drawer.svelte';
  import Badge from '$lib/components/ui/Badge.svelte';

  interface ContainerPort {
    name?: string;
    container_port: number;
    host_port?: number;
    protocol: string;
  }

  interface EnvVar {
    name: string;
    value?: string;
    value_from?: string;
  }

  interface VolumeMount {
    name: string;
    mount_path: string;
    sub_path?: string;
    read_only: boolean;
  }

  interface ProbeInfo {
    probe_type: string;
    handler_type: string;
    details: string;
    initial_delay_seconds: number;
    period_seconds: number;
    timeout_seconds: number;
    success_threshold: number;
    failure_threshold: number;
  }

  interface ContainerInfo {
    name: string;
    image: string;
    image_pull_policy: string;
    ready: boolean;
    restart_count: number;
    state: string;
    cpu_request?: string;
    cpu_limit?: string;
    memory_request?: string;
    memory_limit?: string;
    ports: ContainerPort[];
    env: EnvVar[];
    volume_mounts: VolumeMount[];
    probes: ProbeInfo[];
  }

  interface VolumeInfo {
    name: string;
    volume_type: string;
  }

  interface PodCondition {
    condition_type: string;
    status: string;
    reason?: string;
    message?: string;
    last_transition_time?: string;
  }

  interface PodEventInfo {
    event_type: string;
    reason: string;
    message: string;
    count: number;
    first_timestamp?: string;
    last_timestamp?: string;
    source: string;
  }

  interface Pod {
    name: string;
    namespace: string;
    status: string;
    age: string;
    containers: number;
    restarts: number;
    node: string;
    qos: string;
    controlled_by: string;
    creation_timestamp?: string;
    labels: Record<string, string>;
    annotations: Record<string, string>;
    pod_ip: string;
    host_ip: string;
    service_account: string;
    priority_class: string;
    container_details: ContainerInfo[];
    volumes: VolumeInfo[];
    conditions: PodCondition[];
  }

  let {
    open = $bindable(false),
    pod = $bindable<Pod | null>(null),
    events = $bindable<PodEventInfo[]>([]),
    loadingEvents = $bindable(false),
  }: {
    open: boolean;
    pod: Pod | null;
    events: PodEventInfo[];
    loadingEvents: boolean;
  } = $props();

  function getStatusVariant(status: string): 'success' | 'warning' | 'error' | 'info' | 'neutral' {
    const lower = status.toLowerCase();
    if (lower === 'running' || lower === 'succeeded') return 'success';
    if (lower === 'pending') return 'warning';
    if (lower === 'failed' || lower === 'crashloopbackoff') return 'error';
    return 'info';
  }
</script>

<Drawer bind:open title={pod?.name || 'Pod Details'}>
  {#if pod}
    <div class="space-y-6">
      <!-- Overview Section -->
      <div class="space-y-4">
        <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">Overview</h3>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Namespace</div>
            <div class="text-sm">{pod.namespace}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Status</div>
            <Badge variant={getStatusVariant(pod.status)}>{pod.status}</Badge>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Node</div>
            <div class="text-sm">{pod.node || '-'}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Age</div>
            <div class="text-sm">{pod.age}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">QoS Class</div>
            <div class="text-sm">{pod.qos || '-'}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Controlled By</div>
            <div class="text-sm">{pod.controlled_by}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Pod IP</div>
            <div class="text-sm font-mono">{pod.pod_ip}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Host IP</div>
            <div class="text-sm font-mono">{pod.host_ip}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Service Account</div>
            <div class="text-sm">{pod.service_account}</div>
          </div>
          <div>
            <div class="text-xs text-text-muted uppercase font-semibold mb-1">Priority Class</div>
            <div class="text-sm">{pod.priority_class}</div>
          </div>
        </div>
      </div>

      <!-- Containers Section -->
      <div class="space-y-4">
        <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
          Containers ({pod.container_details?.length || 0})
        </h3>
        {#if pod.container_details && pod.container_details.length > 0}
          <div class="space-y-3">
            {#each pod.container_details as container}
              <div class="p-4 bg-bg-panel rounded-md space-y-2">
                <div class="flex items-center justify-between">
                  <div class="font-semibold">{container.name}</div>
                  <Badge variant={container.ready ? 'success' : 'warning'}>
                    {container.ready ? 'Ready' : 'Not Ready'}
                  </Badge>
                </div>
                <div class="text-xs text-text-muted space-y-1">
                  <div><span class="font-semibold">Image:</span> {container.image}</div>
                  <div><span class="font-semibold">Pull Policy:</span> {container.image_pull_policy}</div>
                  <div><span class="font-semibold">State:</span> {container.state}</div>
                  <div><span class="font-semibold">Restarts:</span> {container.restart_count}</div>
                </div>

                <!-- Ports -->
                {#if container.ports && container.ports.length > 0}
                  <div class="mt-2 pt-2 border-t border-border/50">
                    <div class="text-xs text-text-muted font-semibold mb-2">Ports</div>
                    <div class="space-y-1">
                      {#each container.ports as port}
                        <div class="text-xs font-mono bg-bg-main/50 p-2 rounded">
                          {#if port.name}<span class="text-text-muted">{port.name}:</span> {/if}
                          {port.container_port}
                          {#if port.host_port} → {port.host_port}{/if}
                          <span class="text-text-muted">/{port.protocol}</span>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Environment Variables -->
                {#if container.env && container.env.length > 0}
                  <div class="mt-2 pt-2 border-t border-border/50">
                    <div class="text-xs text-text-muted font-semibold mb-2">Environment ({container.env.length})</div>
                    <div class="space-y-1 max-h-40 overflow-y-auto">
                      {#each container.env as envVar}
                        <div class="text-xs font-mono bg-bg-main/50 p-2 rounded break-all">
                          <span class="text-text-muted font-semibold">{envVar.name}:</span>
                          {#if envVar.value}
                            {envVar.value}
                          {:else if envVar.value_from}
                            <span class="text-text-muted italic">{envVar.value_from}</span>
                          {:else}
                            <span class="text-text-muted">-</span>
                          {/if}
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Volume Mounts -->
                {#if container.volume_mounts && container.volume_mounts.length > 0}
                  <div class="mt-2 pt-2 border-t border-border/50">
                    <div class="text-xs text-text-muted font-semibold mb-2">Mounts ({container.volume_mounts.length})</div>
                    <div class="space-y-1 max-h-40 overflow-y-auto">
                      {#each container.volume_mounts as mount}
                        <div class="text-xs font-mono bg-bg-main/50 p-2 rounded">
                          <div><span class="text-text-muted">Volume:</span> {mount.name}</div>
                          <div><span class="text-text-muted">Path:</span> {mount.mount_path}</div>
                          {#if mount.sub_path}
                            <div><span class="text-text-muted">SubPath:</span> {mount.sub_path}</div>
                          {/if}
                          <div>
                            <Badge variant={mount.read_only ? 'neutral' : 'success'}>
                              {mount.read_only ? 'Read-Only' : 'Read-Write'}
                            </Badge>
                          </div>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Probes -->
                {#if container.probes && container.probes.length > 0}
                  <div class="mt-2 pt-2 border-t border-border/50">
                    <div class="text-xs text-text-muted font-semibold mb-2">Probes</div>
                    <div class="space-y-2">
                      {#each container.probes as probe}
                        <div class="text-xs bg-bg-main/50 p-2 rounded">
                          <div class="font-semibold capitalize mb-1">{probe.probe_type}</div>
                          <div class="text-text-muted">
                            <span class="font-semibold">{probe.handler_type}:</span> {probe.details}
                          </div>
                          <div class="grid grid-cols-2 gap-1 mt-1 text-text-muted">
                            <div>Delay: {probe.initial_delay_seconds}s</div>
                            <div>Period: {probe.period_seconds}s</div>
                            <div>Timeout: {probe.timeout_seconds}s</div>
                            <div>Threshold: {probe.success_threshold}/{probe.failure_threshold}</div>
                          </div>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Resources -->
                {#if container.cpu_request || container.cpu_limit || container.memory_request || container.memory_limit}
                  <div class="mt-2 pt-2 border-t border-border/50">
                    <div class="text-xs text-text-muted font-semibold mb-1">Resources</div>
                    <div class="grid grid-cols-2 gap-2 text-xs">
                      <div>
                        <span class="text-text-muted">CPU Request:</span> {container.cpu_request || '-'}
                      </div>
                      <div>
                        <span class="text-text-muted">CPU Limit:</span> {container.cpu_limit || '-'}
                      </div>
                      <div>
                        <span class="text-text-muted">Memory Request:</span> {container.memory_request || '-'}
                      </div>
                      <div>
                        <span class="text-text-muted">Memory Limit:</span> {container.memory_limit || '-'}
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-sm text-text-muted">No container details available</div>
        {/if}
      </div>

      <!-- Conditions Section -->
      {#if pod.conditions && pod.conditions.length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">Conditions</h3>
          <div class="space-y-2">
            {#each pod.conditions as condition}
              <div class="p-3 bg-bg-panel rounded-md">
                <div class="flex items-center justify-between mb-1">
                  <div class="font-semibold text-sm">{condition.condition_type}</div>
                  <Badge variant={condition.status === 'True' ? 'success' : 'neutral'}>
                    {condition.status}
                  </Badge>
                </div>
                {#if condition.reason}
                  <div class="text-xs text-text-muted"><span class="font-semibold">Reason:</span> {condition.reason}</div>
                {/if}
                {#if condition.message}
                  <div class="text-xs text-text-muted mt-1">{condition.message}</div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Volumes Section -->
      {#if pod.volumes && pod.volumes.length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Volumes ({pod.volumes.length})
          </h3>
          <div class="space-y-2">
            {#each pod.volumes as volume}
              <div class="p-3 bg-bg-panel rounded-md flex items-center justify-between">
                <div class="font-semibold text-sm">{volume.name}</div>
                <Badge variant="neutral">{volume.volume_type}</Badge>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Labels Section -->
      {#if pod.labels && Object.keys(pod.labels).length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Labels ({Object.keys(pod.labels).length})
          </h3>
          <div class="space-y-1">
            {#each Object.entries(pod.labels) as [key, value]}
              <div class="p-2 bg-bg-panel rounded-md text-xs font-mono">
                <span class="text-text-muted">{key}:</span> {value}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Annotations Section -->
      {#if pod.annotations && Object.keys(pod.annotations).length > 0}
        <div class="space-y-4">
          <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
            Annotations ({Object.keys(pod.annotations).length})
          </h3>
          <div class="space-y-1 max-h-64 overflow-y-auto">
            {#each Object.entries(pod.annotations) as [key, value]}
              <div class="p-2 bg-bg-panel rounded-md text-xs font-mono break-all">
                <div class="text-text-muted font-semibold mb-1">{key}</div>
                <div class="text-text">{value}</div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Events Section -->
      <div class="space-y-4">
        <h3 class="text-sm font-bold uppercase text-text-muted border-b border-border pb-2">
          Events {#if events.length > 0}({events.length}){/if}
        </h3>
        {#if loadingEvents}
          <div class="text-sm text-text-muted text-center py-4">Loading events...</div>
        {:else if events.length > 0}
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
  {/if}
</Drawer>
