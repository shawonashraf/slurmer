<script lang="ts">
  import { app } from "./state.svelte";
  import { errorMessage, type Cluster } from "./api";

  let { cluster, onClose }: { cluster: Cluster | null; onClose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let name = $state(cluster?.name ?? "");
  // svelte-ignore state_referenced_locally
  let host = $state(cluster?.host ?? "");
  // svelte-ignore state_referenced_locally
  let user = $state(cluster?.user ?? "");
  let importAlias = $state("");
  let saving = $state(false);
  let confirmingDelete = $state(false);
  let error = $state<string | null>(null);

  const isEdit = $derived(cluster !== null);
  const canSave = $derived(name.trim() !== "" && host.trim() !== "" && !saving);

  function applyImport() {
    const h = app.sshHosts.find((x) => x.alias === importAlias);
    if (!h) return;
    host = h.alias;
    if (h.user) user = h.user;
    if (name.trim() === "") name = h.alias;
  }

  async function save() {
    if (!canSave) return;
    saving = true;
    error = null;
    try {
      await app.saveCluster({ id: cluster?.id, name, host, user });
      onClose();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!cluster) return;
    saving = true;
    error = null;
    try {
      await app.deleteCluster(cluster.id);
      onClose();
    } catch (e) {
      error = errorMessage(e);
      saving = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    if (e.key === "Enter" && !(e.target instanceof HTMLSelectElement)) {
      e.preventDefault();
      save();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="backdrop" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="cluster-form-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <h2 id="cluster-form-title">{isEdit ? "Edit cluster" : "Add cluster"}</h2>

    {#if app.sshHosts.length > 0}
      <div class="field">
        <label for="import">Import from ssh config</label>
        <select id="import" bind:value={importAlias} onchange={applyImport}>
          <option value="">Choose an alias…</option>
          {#each app.sshHosts as h (h.alias)}
            <option value={h.alias}>{h.alias}{h.hostname ? ` (${h.hostname})` : ""}</option>
          {/each}
        </select>
      </div>
    {/if}

    <div class="field">
      <label for="name">Name</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="name" bind:value={name} placeholder="Snellius" autofocus />
    </div>

    <div class="field">
      <label for="host">Host</label>
      <input id="host" bind:value={host} placeholder="ssh alias or hostname" spellcheck="false" />
      <span class="help">Used verbatim as the ssh target, so aliases from ~/.ssh/config apply.</span>
    </div>

    <div class="field">
      <label for="user">User</label>
      <input id="user" bind:value={user} placeholder="leave blank to use the alias's User" spellcheck="false" />
    </div>

    {#if error}
      <div class="strip strip-error"><pre>{error}</pre></div>
    {/if}

    <div class="actions">
      {#if isEdit}
        {#if confirmingDelete}
          <span class="confirm">Delete this cluster?</span>
          <button class="btn btn-danger" onclick={remove} disabled={saving}>Delete</button>
          <button class="btn" onclick={() => (confirmingDelete = false)} disabled={saving}>Keep</button>
        {:else}
          <button class="btn btn-ghost danger-text" onclick={() => (confirmingDelete = true)} disabled={saving}>Delete cluster</button>
        {/if}
      {/if}
      <span class="spacer"></span>
      <button class="btn" onclick={onClose} disabled={saving}>Cancel</button>
      <button class="btn btn-primary" onclick={save} disabled={!canSave}>
        {#if saving}<span class="spinner"></span>{/if}
        Save
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }
  .modal {
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 420px;
    max-width: calc(100vw - 32px);
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  h2 {
    margin: 0;
    font-size: 16px;
  }
  .help {
    font-size: 11px;
    color: var(--muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }
  .spacer {
    flex: 1;
  }
  .confirm {
    color: var(--red);
    font-weight: 600;
  }
  .danger-text {
    color: var(--red);
  }
</style>
