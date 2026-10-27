// Theme vor dem ersten Rendern setzen, um Aufblitzen zu vermeiden.
// Als eigene Datei (statt Inline-Skript), damit die CSP ohne 'unsafe-inline' auskommt.
try {
	const t = localStorage.getItem('akl.theme');
	const dark =
		t === 'dark' || (t !== 'light' && !matchMedia('(prefers-color-scheme: light)').matches);
	document.documentElement.dataset.theme = dark ? 'dark' : 'light';
} catch {
	/* Speicher nicht verfügbar: Dark bleibt Standard */
}
