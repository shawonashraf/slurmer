<script lang="ts">
  import type { Group } from "./jobState";
  import StateBadge from "./StateBadge.svelte";

  let { groups, showCluster }: { groups: Group[]; showCluster: boolean } = $props();

  const columnCount = $derived(showCluster ? 11 : 10);
</script>

<div class="wrap">
  <table>
    <thead>
      <tr>
        {#if showCluster}<th class="cluster">Cluster</th>{/if}
        <th class="id">Job ID</th>
        <th>Partition</th>
        <th class="name">Name</th>
        <th>State</th>
        <th class="num">Time</th>
        <th class="num">Nodes</th>
        <th>Reason</th>
        <th class="num">Time Limit</th>
        <th class="num">CPUs</th>
        <th class="num">Min Memory</th>
      </tr>
    </thead>
    <tbody>
      {#each groups as g (g.cluster.id)}
        {#if g.error}
          <tr class="note">
            <td colspan={columnCount}>
              <div class="strip strip-error">
                {#if showCluster}<strong>{g.cluster.name}:</strong>{/if}
                <pre>{g.error}</pre>
                <span class="hint">Open a terminal and run <code>ssh {g.cluster.host}</code> once to establish a session, then retry.</span>
              </div>
            </td>
          </tr>
        {:else if !g.fetched}
          <tr class="note">
            <td colspan={columnCount} class="muted">
              {#if showCluster}<strong>{g.cluster.name}:</strong>{/if} not refreshed yet
            </td>
          </tr>
        {:else if g.jobs.length === 0}
          <tr class="note">
            <td colspan={columnCount} class="muted">
              {#if showCluster}<strong>{g.cluster.name}:</strong>{/if} no jobs in the queue
            </td>
          </tr>
        {:else}
          {#each g.jobs as job (g.cluster.id + ":" + job.id)}
            <tr>
              {#if showCluster}<td class="cluster">{g.cluster.name}</td>{/if}
              <td class="id mono">{job.id}</td>
              <td>{job.partition}</td>
              <td class="name" title={job.name}>{job.name}</td>
              <td><StateBadge state={job.state} /></td>
              <td class="num tabular">{job.time}</td>
              <td class="num">{job.nodes}</td>
              <td class="reason" title={job.reason}>{job.reason}</td>
              <td class="num tabular">{job.time_limit}</td>
              <td class="num">{job.cpus}</td>
              <td class="num">{job.min_memory}</td>
            </tr>
          {/each}
        {/if}
      {/each}
    </tbody>
  </table>
</div>

<style>
  .wrap {
    height: 100%;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    table-layout: fixed;
    min-width: 900px;
  }
  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 8px 10px;
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  td {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  tbody tr:not(.note):hover td {
    background: var(--surface-hover);
  }
  th.cluster, td.cluster { width: 120px; }
  th.id, td.id { width: 100px; }
  th.name, td.name { width: auto; }
  th.num, td.num { width: 90px; text-align: right; }
  td.reason { width: 140px; }
  .mono { font-family: var(--mono); font-size: 12px; }
  .tabular { font-variant-numeric: tabular-nums; }
  .muted { color: var(--muted); white-space: normal; }
  tr.note td { white-space: normal; }
  tr.note .strip { margin: 2px 0; }
  code { font-family: var(--mono); }
</style>
