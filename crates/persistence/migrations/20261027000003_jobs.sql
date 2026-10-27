-- AKL-M060-01/03: persistente Job-Queue.
-- Payload enthält nur Referenzen/Parameter, niemals Medien oder Secrets.

CREATE TABLE jobs (
    id              UUID PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE CHECK (char_length(idempotency_key) BETWEEN 1 AND 200),
    kind            TEXT NOT NULL CHECK (char_length(kind) BETWEEN 1 AND 64),
    state           TEXT NOT NULL DEFAULT 'queued' CHECK (state IN (
                        'queued', 'running', 'completed', 'failed', 'retryable', 'uncertain', 'cancelled')),
    -- Fortschritt innerhalb eines Laufs; 'callback' bedeutet: Nebenwirkung nach außen begonnen.
    phase           TEXT NOT NULL DEFAULT 'pending' CHECK (phase IN ('pending', 'processing', 'callback')),
    priority        SMALLINT NOT NULL DEFAULT 0,
    attempts        INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts    INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts BETWEEN 1 AND 20),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lease_until     TIMESTAMPTZ,
    worker_id       TEXT,
    last_error_code TEXT,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (octet_length(payload::text) <= 16384),
    result          JSONB CHECK (octet_length(result::text) <= 16384),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ
);

CREATE INDEX jobs_ready ON jobs (priority DESC, created_at) WHERE state IN ('queued', 'retryable');
CREATE INDEX jobs_running_lease ON jobs (lease_until) WHERE state = 'running';
CREATE INDEX jobs_state_created ON jobs (state, created_at DESC);

-- Jeder Start zählt für das dauerhafte, globale Stundenlimit (überlebt Neustarts).
CREATE TABLE job_starts (
    id         BIGSERIAL PRIMARY KEY,
    job_id     UUID NOT NULL REFERENCES jobs (id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX job_starts_recent ON job_starts (started_at DESC);
