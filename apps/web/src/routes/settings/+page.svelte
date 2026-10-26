<script lang="ts">
	import { i18n, LOCALES, t, type Locale } from '#lib/i18n/index.svelte.ts';
	import { theme, type ThemePreference } from '#lib/stores/theme.svelte.ts';

	const themes: ThemePreference[] = ['system', 'dark', 'light'];
	const localeNames: Record<Locale, string> = { de: 'Deutsch', en: 'English' };
</script>

<section
	class="max-w-2xl rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="appearance"
>
	<div class="border-b border-line px-4 py-3">
		<h2 id="appearance" class="text-sm font-semibold">{t('settings.appearance')}</h2>
		<p class="mt-0.5 text-xs text-faint">{t('settings.localNote')}</p>
	</div>
	<div class="divide-y divide-line">
		<fieldset class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
			<legend class="sr-only">{t('settings.theme')}</legend>
			<span class="text-[13px]" aria-hidden="true">{t('settings.theme')}</span>
			<div class="inline-flex rounded-lg border border-line bg-surface-2 p-0.5">
				{#each themes as option (option)}
					<label
						class="cursor-pointer rounded-md px-3 py-1 text-xs has-checked:bg-surface has-checked:text-fg has-checked:shadow-card text-muted has-focus-visible:outline-2 has-focus-visible:outline-accent"
					>
						<input
							type="radio"
							name="theme"
							value={option}
							class="sr-only"
							checked={theme.preference === option}
							onchange={() => theme.set(option)}
						/>
						{t(`settings.theme.${option}`)}
					</label>
				{/each}
			</div>
		</fieldset>
		<div class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
			<label for="locale" class="text-[13px]">{t('settings.language')}</label>
			<select
				id="locale"
				class="rounded-md border border-line bg-surface-2 px-2 py-1 text-[13px]"
				value={i18n.locale}
				onchange={(e) => i18n.set(e.currentTarget.value as Locale)}
			>
				{#each LOCALES as code (code)}
					<option value={code}>{localeNames[code]}</option>
				{/each}
			</select>
		</div>
	</div>
</section>
