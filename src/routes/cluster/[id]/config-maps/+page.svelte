<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="ConfigMaps" listCommand="cluster_list_config_maps" deleteCommand="cluster_delete_config_map" />
