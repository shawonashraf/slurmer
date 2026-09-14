<script lang="ts">
  import { app } from "./state.svelte";
  import type { Cluster } from "./api";

  let { onAdd, onEdit }: { onAdd: () => void; onEdit: (c: Cluster) => void } = $props();

  const isAll = $derived(app.selection.kind === "all");

  function isSelected(c: Cluster): boolean {
    return app.selection.kind === "cluster" && app.selection.id === c.id;
  }
</script>

<aside class="sidebar">
  <div class="brand">slurmer</div>

  <nav>
    <button class="item" class:active={isAll} onclick={() => app.select({ kind: "all" })}>
      <span class="icon" aria-hidden="true">▦</span>
      <span class="label">All clusters</span>
    </button>

    <div class="section">Clusters</div>

    {#if app.booted && app.clusters.length === 0}
      <div class="none">No clusters yet</div>
    {/if}

    {#each app.clusters as c (c.id)}
      <div class="row" class:active={isSelected(c)}>
        <button class="item" onclick={() => app.select({ kind: "cluster", id: c.id })} title={c.host}>
          <span class="icon" aria-hidden="true">
            {#if app.loading[c.id]}
              <span class="spinner small"></span>
            {:else if app.errors[c.id]}
              <span class="dot error"></span>
            {:else if app.fetched[c.id]}
              <span class="dot ok"></span>
            {:else}
              <span class="dot"></span>
            {/if}
          </span>
          <span class="label">{c.name}</span>
        </button>
        <button class="edit" title="Edit cluster" aria-label="Edit {c.name}" onclick={() => onEdit(c)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20h9" />
            <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
          </svg>
        </button>
      </div>
    {/each}
  </nav>

  <button class="btn add" onclick={onAdd}>+ Add cluster</button>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 220px;
    flex: none;
    height: 100%;
    padding: 12px;
    background: var(--surface-2);
    border-right: 1px solid var(--border);
  }
  .brand {
    padding: 6px 8px 14px;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.2px;
  }
  nav {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .section {
    margin: 14px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--muted);
  }
  .none {
    padding: 6px 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    height: 30px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .item:hover,
  .row:hover .item {
    background: var(--surface-hover);
  }
  .item.active,
  .row.active .item {
    background: var(--accent);
    color: var(--accent-text);
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    flex: none;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
  }
  .dot.ok {
    background: var(--green);
  }
  .dot.error {
    background: var(--red);
  }
  .spinner.small {
    width: 10px;
    height: 10px;
  }
  .edit {
    position: absolute;
    right: 4px;
    display: none;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .row:hover .edit, .row:focus-within .edit {
    display: inline-flex;
  }
  .row.active .edit {
    color: var(--accent-text);
  }
  .edit:hover {
    background: rgba(127, 127, 127, 0.25);
  }
  .edit:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .add {
    margin-top: 12px;
    justify-content: center;
  }
</style>
