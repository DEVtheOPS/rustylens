<script lang="ts">
 import WorkloadList from "$lib/components/WorkloadList.svelte";
</script>
<WorkloadList title="ResourceQuotas" listCommand="cluster_list_resource_quotas" deleteCommand="cluster_delete_resource_quota" />
