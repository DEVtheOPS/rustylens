<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="ServiceAccounts" listCommand="cluster_list_service_accounts" deleteCommand="cluster_delete_service_account" />
