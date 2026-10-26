import { error } from '@sveltejs/kit';
import { NAV } from '#lib/nav.ts';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ params }) => {
	const item = NAV.find((i) => i.href === `/${params.section}` && !i.implemented);
	if (!item) error(404, 'Not found');
	return { item };
};
