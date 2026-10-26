import { de, type MessageKey } from './de';
import { en } from './en';

export type Locale = 'de' | 'en';
export const LOCALES: readonly Locale[] = ['de', 'en'];
const dictionaries: Record<Locale, Record<MessageKey, string>> = { de, en };

const STORAGE_KEY = 'akl.locale';

function initialLocale(): Locale {
	try {
		const stored = globalThis.localStorage?.getItem(STORAGE_KEY);
		if (stored === 'de' || stored === 'en') return stored;
	} catch {
		/* Speicher nicht verfügbar */
	}
	return 'de';
}

class I18n {
	locale = $state<Locale>(initialLocale());

	set(locale: Locale) {
		this.locale = locale;
		try {
			localStorage.setItem(STORAGE_KEY, locale);
		} catch {
			/* nur Komfort */
		}
		document.documentElement.lang = locale;
	}

	t = (key: MessageKey): string => dictionaries[this.locale][key] ?? de[key];
}

export const i18n = new I18n();
export const t = (key: MessageKey) => i18n.t(key);
export { type MessageKey };
