-- AKL-M060-02/05/06: Modell-Provider und gemessene Testergebnisse.
-- API-Keys werden nie gespeichert, nur der Name einer Secret-Datei.

CREATE TABLE providers (
    id             UUID PRIMARY KEY,
    name           TEXT NOT NULL UNIQUE CHECK (char_length(name) BETWEEN 1 AND 64),
    kind           TEXT NOT NULL CHECK (kind IN ('ollama', 'openai_compatible')),
    base_url       TEXT NOT NULL CHECK (char_length(base_url) <= 512),
    api_key_secret TEXT CHECK (api_key_secret ~ '^[a-z0-9_-]{1,64}$'),
    default_model  TEXT CHECK (char_length(default_model) <= 128),
    enabled        BOOLEAN NOT NULL DEFAULT true,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE provider_tests (
    id          BIGSERIAL PRIMARY KEY,
    provider_id UUID NOT NULL REFERENCES providers (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('connection', 'vision')),
    ok          BOOLEAN NOT NULL,
    latency_ms  BIGINT,
    error_code  TEXT,
    model       TEXT,
    -- Modellantwort gekürzt (max. 500 Zeichen), nur für synthetische Testbilder.
    result_text TEXT CHECK (char_length(result_text) <= 500),
    matched     BOOLEAN,
    tested_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX provider_tests_latest ON provider_tests (provider_id, kind, tested_at DESC);
