<script lang="ts">
  import { toneFor } from "./jobState";

  let { state }: { state: string } = $props();
  const tone = $derived(toneFor(state));
</script>

<span class="badge {tone}">
  <span class="dot"></span>
  {state}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.3px;
    white-space: nowrap;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .green { color: var(--green); background: var(--green-bg); }
  .amber { color: var(--amber); background: var(--amber-bg); }
  .red { color: var(--red); background: var(--red-bg); }
  .blue { color: var(--blue); background: var(--blue-bg); }
  .grey { color: var(--grey); background: var(--grey-bg); }
</style>
