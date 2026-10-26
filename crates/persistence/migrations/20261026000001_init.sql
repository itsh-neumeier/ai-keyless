-- AKL-M010: Grundschema.
-- Keine Geheimnisse in Tabellen: Zugangsdaten liegen ausschließlich in
-- Docker-Secrets bzw. geschützten Dateien und werden hier nur referenziert.

CREATE TABLE device_profiles (
    id           UUID PRIMARY KEY,
    kind         TEXT NOT NULL CHECK (kind IN ('ai_key', 'ai_port')),
    state        TEXT NOT NULL DEFAULT 'unconfigured' CHECK (state IN (
                     'unconfigured', 'discovered', 'pending_adoption', 'adopted',
                     'online', 'reconnecting', 'offline', 'error', 'disabled')),
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Strikte Trennung: höchstens ein aktives AI-Key-Profil pro Installation.
CREATE UNIQUE INDEX device_profiles_single_ai_key
    ON device_profiles (kind) WHERE kind = 'ai_key' AND state <> 'disabled';

CREATE TABLE device_state_transitions (
    id             BIGSERIAL PRIMARY KEY,
    profile_id     UUID NOT NULL REFERENCES device_profiles (id) ON DELETE CASCADE,
    from_state     TEXT NOT NULL,
    to_state       TEXT NOT NULL,
    reason         TEXT,
    correlation_id TEXT,
    occurred_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX device_state_transitions_profile
    ON device_state_transitions (profile_id, occurred_at DESC);

CREATE TABLE audit_log (
    id             BIGSERIAL PRIMARY KEY,
    actor          TEXT NOT NULL,
    action         TEXT NOT NULL,
    target         TEXT,
    details        JSONB NOT NULL DEFAULT '{}'::jsonb,
    correlation_id TEXT,
    occurred_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
