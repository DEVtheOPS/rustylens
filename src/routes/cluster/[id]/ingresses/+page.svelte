<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="Ingresses" listCommand="cluster_list_ingresses" deleteCommand="cluster_delete_ingress" />
