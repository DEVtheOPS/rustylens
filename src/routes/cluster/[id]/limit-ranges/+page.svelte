<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="LimitRanges" listCommand="cluster_list_limit_ranges" deleteCommand="cluster_delete_limit_range" />
