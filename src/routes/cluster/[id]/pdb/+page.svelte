<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="PodDisruptionBudgets" listCommand="cluster_list_pdb" deleteCommand="cluster_delete_pdb" />
