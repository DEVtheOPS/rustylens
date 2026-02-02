<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="NetworkPolicies" listCommand="cluster_list_network_policies" deleteCommand="cluster_delete_network_policy" />
