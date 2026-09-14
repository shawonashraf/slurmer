<script lang="ts">
  import type { Cluster } from "./api";

  let { cluster, onClose }: { cluster: Cluster | null; onClose: () => void } = $props();
</script>

<div>
  form for {cluster?.name ?? "new cluster"}
  <button onclick={onClose}>close</button>
</div>
