import type { MessageKey } from './i18n/de';
import type { IconName } from './components/icons';

export interface NavItem {
	id: string;
	/** Pfad ohne Basis; Links werden in der UI per resolve() erzeugt. */
	href: string;
	label: MessageKey;
	icon: IconName;
	/** Meilenstein, der den Bereich liefert (docs/roadmap/BACKLOG.md). */
	milestone: string;
	/** Nur `true`, wenn der Bereich echte Backend-Daten anzeigt. */
	implemented: boolean;
	group: 'main' | 'admin';
}

export const NAV: readonly NavItem[] = [
	{
		id: 'overview',
		href: '/',
		label: 'nav.overview',
		icon: 'overview',
		milestone: 'AKL-M010',
		implemented: true,
		group: 'main'
	},
	{
		id: 'devices',
		href: '/devices',
		label: 'nav.devices',
		icon: 'device',
		milestone: 'AKL-M040',
		implemented: true,
		group: 'main'
	},
	{
		id: 'cameras',
		href: '/cameras',
		label: 'nav.cameras',
		icon: 'camera',
		milestone: 'AKL-M050',
		implemented: false,
		group: 'main'
	},
	{
		id: 'detections',
		href: '/detections',
		label: 'nav.detections',
		icon: 'detection',
		milestone: 'AKL-M070',
		implemented: false,
		group: 'main'
	},
	{
		id: 'search',
		href: '/search',
		label: 'nav.search',
		icon: 'search',
		milestone: 'AKL-M090',
		implemented: false,
		group: 'main'
	},
	{
		id: 'models',
		href: '/models',
		label: 'nav.models',
		icon: 'model',
		milestone: 'AKL-M060',
		implemented: true,
		group: 'main'
	},
	{
		id: 'processing',
		href: '/processing',
		label: 'nav.processing',
		icon: 'queue',
		milestone: 'AKL-M060',
		implemented: true,
		group: 'main'
	},
	{
		id: 'alerts',
		href: '/alerts',
		label: 'nav.alerts',
		icon: 'bell',
		milestone: 'AKL-M100',
		implemented: false,
		group: 'main'
	},
	{
		id: 'system',
		href: '/system',
		label: 'nav.system',
		icon: 'system',
		milestone: 'AKL-M010',
		implemented: true,
		group: 'admin'
	},
	{
		id: 'settings',
		href: '/settings',
		label: 'nav.settings',
		icon: 'settings',
		milestone: 'AKL-M020',
		implemented: true,
		group: 'admin'
	}
];

export function findNav(pathname: string): NavItem | undefined {
	if (pathname === '/' || pathname === '') return NAV[0];
	return NAV.find((item) => item.href !== '/' && pathname.startsWith(item.href));
}
