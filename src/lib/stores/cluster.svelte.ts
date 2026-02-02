import { invoke } from '@tauri-apps/api/core';

class ClusterStore {
  active = $state<string>('');
  list = $state<string[]>([]);
  namespaces = $state<string[]>([]);
  activeNamespace = $state<string>('all');
  loading = $state(false);

  constructor() {
    // defer refresh to onMount in layout
  }

  async refresh() {
    this.loading = true;
    try {
      const contexts = await invoke<string[]>('list_contexts');
      if (contexts && contexts.length > 0) {
        this.list = contexts;
        if (!this.active || !this.list.includes(this.active)) {
          this.active = this.list[0];
        }
        await this.fetchNamespaces();
      } else {
        this.list = [];
      }
    } catch (e) {
      console.error("Failed to load clusters", e);
    } finally {
      this.loading = false;
    }
  }

  setActive(cluster: string) {
    this.active = cluster;
    this.fetchNamespaces();
  }

  setNamespace(ns: string) {
    this.activeNamespace = ns;
  }

  async fetchNamespaces() {
    if (!this.active) return;
    try {
      // We need to implement list_namespaces in backend
      const nss = await invoke<string[]>('list_namespaces', { contextName: this.active });
      this.namespaces = nss.sort();
      if (!this.namespaces.includes(this.activeNamespace) && this.activeNamespace !== 'all') {
        this.activeNamespace = 'all';
      }
    } catch (e) {
      console.error("Failed to fetch namespaces", e);
      // Fallback
      this.namespaces = ['default', 'kube-system', 'kube-public'];
    }
  }
}

export const clusterStore = new ClusterStore();
