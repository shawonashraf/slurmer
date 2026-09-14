import { invoke } from "@tauri-apps/api/core";

export interface Cluster {
  id: string;
  name: string;
  host: string;
  user: string;
}

export type Selection = { kind: "all" } | { kind: "cluster"; id: string };

export interface ClustersResponse {
  clusters: Cluster[];
  selected: Selection | null;
  load_error: string | null;
}

export interface ClusterInput {
  id?: string;
  name: string;
  host: string;
  user: string;
}

export interface SshHost {
  alias: string;
  hostname: string | null;
  user: string | null;
}

export interface Job {
  id: string;
  partition: string;
  name: string;
  state: string;
  time: string;
  nodes: string;
  reason: string;
  time_limit: string;
  cpus: string;
  min_memory: string;
}

export interface FetchError {
  message: string;
}

export const api = {
  listClusters: () => invoke<ClustersResponse>("list_clusters"),
  saveCluster: (input: ClusterInput) => invoke<Cluster>("save_cluster", { input }),
  deleteCluster: (id: string) => invoke<void>("delete_cluster", { id }),
  setSelection: (selection: Selection | null) => invoke<void>("set_selection", { selection }),
  listSshHosts: () => invoke<SshHost[]>("list_ssh_hosts"),
  fetchJobs: (cluster_id: string) => invoke<Job[]>("fetch_jobs", { cluster_id }),
};

/** Normalises whatever `invoke` rejected with into a display string. */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) {
    const m = (e as { message: unknown }).message;
    if (typeof m === "string") return m;
  }
  return String(e);
}
