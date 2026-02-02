<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="StorageClasses" listCommand="cluster_list_storage_classes" deleteCommand="cluster_delete_storage_class" />
