import type { Cluster, Job } from "./api";

export type Tone = "green" | "amber" | "red" | "blue" | "grey";

const GREEN = new Set(["RUNNING", "COMPLETING"]);
const AMBER = new Set(["PENDING", "CONFIGURING", "SUSPENDED"]);
const RED = new Set(["FAILED", "CANCELLED", "TIMEOUT", "NODE_FAIL", "OUT_OF_MEMORY"]);
const BLUE = new Set(["COMPLETED"]);

/** Maps a Slurm job state (as printed by squeue %T) to a badge colour. */
export function toneFor(state: string): Tone {
  const s = state.trim().toUpperCase();
  if (GREEN.has(s)) return "green";
  if (AMBER.has(s)) return "amber";
  if (RED.has(s)) return "red";
  if (BLUE.has(s)) return "blue";
  return "grey";
}

/** One cluster's slice of the table: its rows plus fetch status. */
export interface Group {
  cluster: Cluster;
  jobs: Job[];
  /** Set only in the combined view; rendered as an inline strip in place of rows. */
  error: string | null;
  fetched: boolean;
}
