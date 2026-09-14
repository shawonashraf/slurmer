<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "./lib/state.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import MainPanel from "./lib/MainPanel.svelte";
  import ClusterForm from "./lib/ClusterForm.svelte";
  import type { Cluster } from "./lib/api";

  let formOpen = $state(false);
  let editing = $state<Cluster | null>(null);

  function openAdd() {
    editing = null;
    formOpen = true;
  }
  function openEdit(c: Cluster) {
    editing = c;
    formOpen = true;
  }
  function closeForm() {
    formOpen = false;
    editing = null;
  }

  onMount(() => {
    app.boot();
  });
</script>

<div class="layout">
  <Sidebar onAdd={openAdd} onEdit={openEdit} />
  <MainPanel onAdd={openAdd} />
</div>

{#if formOpen}
  <ClusterForm cluster={editing} onClose={closeForm} />
{/if}

<style>
  .layout {
    display: flex;
    height: 100%;
    background: var(--bg);
  }
</style>
