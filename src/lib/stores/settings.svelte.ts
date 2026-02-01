export type Theme = 'kore' | 'kore-light' | 'rusty' | 'rusty-light' | 'dracula' | 'alucard';

export interface Settings {
  theme: Theme;
  refreshInterval: number;
}

class SettingsStore {
  value = $state<Settings>({
    theme: 'kore',
    refreshInterval: 5000,
  });

  constructor() {
    if (typeof localStorage !== 'undefined') {
      const stored = localStorage.getItem('app-settings');
      if (stored) {
        try {
          const parsed = JSON.parse(stored);
          this.value = { ...this.value, ...parsed };
        } catch (e) {
          console.error("Failed to load settings", e);
        }
      }
    }
  }

  setTheme(theme: Theme) {
    this.value.theme = theme;
    this.save();
  }

  setRefreshInterval(ms: number) {
    this.value.refreshInterval = ms;
    this.save();
  }

  save() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('app-settings', JSON.stringify(this.value));
    }
  }
}

export const settingsStore = new SettingsStore();
