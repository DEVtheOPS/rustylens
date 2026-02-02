<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="HorizontalPodAutoscalers" listCommand="cluster_list_hpa" deleteCommand="cluster_delete_hpa" />
