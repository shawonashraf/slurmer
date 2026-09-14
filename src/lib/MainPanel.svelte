<script lang="ts">
  import { app } from "./state.svelte";
  import JobTable from "./JobTable.svelte";
  import type { Group } from "./jobState";

  let { onAdd }: { onAdd: () => void } = $props();

  const selected = $derived(app.selectedCluster);
  const isAll = $derived(app.selection.kind === "all");

  const title = $derived(isAll ? "All clusters" : (selected?.name ?? ""));
  const subtitle = $derived.by(() => {
    if (isAll) {
      const n = app.clusters.length;
      return n === 1 ? "1 cluster" : `${n} clusters`;
    }
    if (!selected) return "";
    return selected.user ? `${selected.user}@${selected.host}` : selected.host;
  });

  const busy = $derived(isAll ? app.anyLoading : selected ? Boolean(app.loading[selected.id]) : false);

  const groups: Group[] = $derived.by(() => {
    const list = isAll ? app.clusters : selected ? [selected] : [];
    return list.map((cluster) => ({
      cluster,
      jobs: app.jobs[cluster.id] ?? [],
      // In the single view the error is shown as a strip above the table,
      // so the group itself carries no error there.
      error: isAll ? (app.errors[cluster.id] ?? null) : null,
      fetched: Boolean(app.fetched[cluster.id]),
    }));
  });

  const singleError = $derived(!isAll && selected ? (app.errors[selected.id] ?? null) : null);
  const anyFetched = $derived(groups.some((g) => g.fetched));
  const singleEmpty = $derived(
    !isAll && selected ? app.fetched[selected.id] && (app.jobs[selected.id]?.length ?? 0) === 0 && !singleError : false,
  );
</script>

<section class="main">
  <header class="toolbar">
    <div class="titles">
      <h1>{title}</h1>
      {#if subtitle}<div class="subtitle">{subtitle}</div>{/if}
    </div>
    {#if app.clusters.length > 0}
      <button class="btn btn-primary" onclick={() => app.refreshSelected()} disabled={busy}>
        {#if busy}
          <span class="spinner"></span>
          Refreshing
        {:else}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M21 12a9 9 0 1 1-2.64-6.36" />
            <path d="M21 3v6h-6" />
          </svg>
          Refresh
        {/if}
      </button>
    {/if}
  </header>

  {#if app.loadError}
    <div class="strip strip-warn notice">
      <span>{app.loadError}</span>
      <button class="btn btn-ghost" onclick={() => app.dismissLoadError()}>Dismiss</button>
    </div>
  {/if}

  {#if singleError && selected}
    <div class="strip strip-error notice-block">
      <pre>{singleError}</pre>
      <span class="hint">Open a terminal and run <code>ssh {selected.host}</code> once to establish a session, then retry.</span>
    </div>
  {/if}

  <div class="content" class:dimmed={busy}>
    {#if app.clusters.length === 0}
      <div class="empty">
        <h2>No clusters yet</h2>
        <p>Add a cluster to start tracking its Slurm queue.</p>
        <button class="btn btn-primary" onclick={onAdd}>Add cluster</button>
      </div>
    {:else if !anyFetched}
      <div class="empty">
        <h2>Press Refresh to load jobs</h2>
        <p>{isAll ? "Every cluster is refreshed in parallel." : subtitle}</p>
      </div>
    {:else if singleEmpty}
      <div class="empty">
        <h2>No jobs in the queue</h2>
        <p>Nothing running or pending for this user.</p>
      </div>
    {:else}
      <JobTable {groups} showCluster={isAll} />
    {/if}
  </div>
</section>

<style>
  .main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 16px 20px;
    gap: 12px;
  }
  .toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }
  .subtitle {
    margin-top: 2px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 12px;
  }
  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .notice-block code {
    font-family: var(--mono);
  }
  .content {
    flex: 1;
    min-height: 0;
    transition: opacity 0.15s ease;
  }
  .content.dimmed {
    opacity: 0.55;
    pointer-events: none;
  }
  .empty p {
    margin: 0;
  }
</style>
