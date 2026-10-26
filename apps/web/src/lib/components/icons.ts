// Eigene, schlichte Linien-Icons (24×24, stroke). Keine Hersteller-Icons.
export const ICONS = {
	overview: 'M4 4h7v7H4zM13 4h7v4h-7zM13 10h7v10h-7zM4 13h7v7H4z',
	device: 'M5 7h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V8a1 1 0 0 1 1-1zM8 12h.01M12 12h4',
	camera: 'M4 8h11a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1zM16 11l5-3v8l-5-3',
	detection:
		'M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3M9 12a3 3 0 1 0 6 0 3 3 0 1 0-6 0',
	search: 'M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 1 0 0-13zM15.5 15.5 20 20',
	model: 'M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 12l8-4.5M12 12v9M12 12 4 7.5',
	queue: 'M4 6h16M4 12h16M4 18h10',
	bell: 'M6 16V11a6 6 0 1 1 12 0v5l1.5 2h-15zM10 20.5a2 2 0 0 0 4 0',
	system: 'M4 5h16v10H4zM9 19h6M12 15v4',
	settings:
		'M12 9a3 3 0 1 0 0 6 3 3 0 1 0 0-6zM12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8',
	chevronLeft: 'M14.5 6 8.5 12l6 6',
	chevronRight: 'M9.5 6l6 6-6 6',
	info: 'M12 3a9 9 0 1 0 0 18 9 9 0 1 0 0-18zM12 11v5M12 8h.01',
	database:
		'M5 6c0-1.7 3.1-3 7-3s7 1.3 7 3-3.1 3-7 3-7-1.3-7-3zM5 6v12c0 1.7 3.1 3 7 3s7-1.3 7-3V6M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3',
	api: 'M8 7 3 12l5 5M16 7l5 5-5 5M13.5 5l-3 14'
} as const;

export type IconName = keyof typeof ICONS;
