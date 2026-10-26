export type ProviderKind = 'ollama' | 'openai_compatible';

export interface ProviderTest {
	kind: 'connection' | 'vision';
	ok: boolean;
	latency_ms: number | null;
	error_code: string | null;
	model: string | null;
	result_text: string | null;
	matched: boolean | null;
	tested_at: string;
}

export interface ProviderView {
	id: string;
	name: string;
	kind: ProviderKind;
	base_url: string;
	api_key_secret: string | null;
	default_model: string | null;
	enabled: boolean;
	created_at: string;
	tests: ProviderTest[];
}

export interface ConnectionResult {
	ok: boolean;
	latency_ms: number | null;
	models: string[];
	error_code: string | null;
}
