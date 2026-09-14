import {
  api,
  errorMessage,
  type Cluster,
  type ClusterInput,
  type Job,
  type Selection,
  type SshHost,
} from "./api";

class AppState {
  clusters = $state<Cluster[]>([]);
  sshHosts = $state<SshHost[]>([]);
  selection = $state<Selection>({ kind: "all" });
  /** Last successful fetch per cluster id. */
  jobs = $state<Record<string, Job[]>>({});
  /** Last fetch error per cluster id; absent after a successful fetch. */
  errors = $state<Record<string, string>>({});
  loading = $state<Record<string, boolean>>({});
  /** True once a cluster has been refreshed at least once (success or error). */
  fetched = $state<Record<string, boolean>>({});
  loadError = $state<string | null>(null);
  booted = $state(false);

  get selectedCluster(): Cluster | null {
    if (this.selection.kind !== "cluster") return null;
    const id = this.selection.id;
    return this.clusters.find((c) => c.id === id) ?? null;
  }

  async boot(): Promise<void> {
    const [res, hosts] = await Promise.all([api.listClusters(), api.listSshHosts()]);
    this.clusters = res.clusters;
    this.loadError = res.load_error;
    this.sshHosts = hosts;

    const saved = res.selected;
    if (saved?.kind === "cluster" && this.clusters.some((c) => c.id === saved.id)) {
      this.selection = saved;
    } else if (saved?.kind === "all") {
      this.selection = saved;
    } else if (this.clusters.length > 0) {
      this.selection = { kind: "cluster", id: this.clusters[0].id };
    } else {
      this.selection = { kind: "all" };
    }
    this.booted = true;
  }

  async select(selection: Selection): Promise<void> {
    this.selection = selection;
    try {
      await api.setSelection(selection);
    } catch {
      // Persisting the selection is a convenience; failing is not fatal.
    }
  }

  async refresh(id: string): Promise<void> {
    this.loading[id] = true;
    try {
      this.jobs[id] = await api.fetchJobs(id);
      delete this.errors[id];
    } catch (e) {
      this.errors[id] = errorMessage(e);
    } finally {
      this.loading[id] = false;
      this.fetched[id] = true;
    }
  }

  async refreshAll(): Promise<void> {
    await Promise.allSettled(this.clusters.map((c) => this.refresh(c.id)));
  }

  async refreshSelected(): Promise<void> {
    if (this.selection.kind === "all") await this.refreshAll();
    else await this.refresh(this.selection.id);
  }

  get anyLoading(): boolean {
    return Object.values(this.loading).some(Boolean);
  }

  async saveCluster(input: ClusterInput): Promise<Cluster> {
    const saved = await api.saveCluster(input);
    const i = this.clusters.findIndex((c) => c.id === saved.id);
    if (i >= 0) this.clusters[i] = saved;
    else this.clusters.push(saved);
    // A successful save rewrote the file, so a stale load error no longer applies.
    this.loadError = null;
    return saved;
  }

  async deleteCluster(id: string): Promise<void> {
    await api.deleteCluster(id);
    this.clusters = this.clusters.filter((c) => c.id !== id);
    delete this.jobs[id];
    delete this.errors[id];
    delete this.loading[id];
    delete this.fetched[id];
    if (this.selection.kind === "cluster" && this.selection.id === id) {
      await this.select(
        this.clusters.length > 0 ? { kind: "cluster", id: this.clusters[0].id } : { kind: "all" },
      );
    }
  }

  dismissLoadError(): void {
    this.loadError = null;
  }
}

export const app = new AppState();
