<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="PersistentVolumeClaims" listCommand="cluster_list_pvc" deleteCommand="cluster_delete_pvc" />
