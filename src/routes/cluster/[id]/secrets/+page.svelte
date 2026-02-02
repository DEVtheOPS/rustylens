<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="Secrets" listCommand="cluster_list_secrets" deleteCommand="cluster_delete_secret" />
