<script lang="ts">
  import type { Group } from "./jobState";

  let { groups, showCluster }: { groups: Group[]; showCluster: boolean } = $props();
</script>

<div>{groups.length} groups, showCluster={showCluster}</div>
