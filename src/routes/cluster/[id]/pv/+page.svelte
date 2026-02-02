<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="PersistentVolumes" listCommand="cluster_list_pv" deleteCommand="cluster_delete_pv" />
