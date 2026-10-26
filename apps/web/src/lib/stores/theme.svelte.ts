export type ThemePreference = 'system' | 'dark' | 'light';
const STORAGE_KEY = 'akl.theme';

function stored(): ThemePreference {
	try {
		const v = globalThis.localStorage?.getItem(STORAGE_KEY);
		if (v === 'dark' || v === 'light') return v;
	} catch {
		/* Speicher nicht verfügbar */
	}
	return 'system';
}

class Theme {
	preference = $state<ThemePreference>(stored());

	set(pref: ThemePreference) {
		this.preference = pref;
		try {
			if (pref === 'system') localStorage.removeItem(STORAGE_KEY);
			else localStorage.setItem(STORAGE_KEY, pref);
		} catch {
			/* nur Komfort */
		}
		this.apply();
	}

	apply() {
		const dark =
			this.preference === 'dark' ||
			(this.preference === 'system' && !matchMedia('(prefers-color-scheme: light)').matches);
		document.documentElement.dataset.theme = dark ? 'dark' : 'light';
	}
}

export const theme = new Theme();
