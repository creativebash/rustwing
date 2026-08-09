CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    job_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PENDING', 'RUNNING', 'RETRY_SCHEDULED', 'COMPLETED', 'DEAD')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL CHECK (max_attempts > 0),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at TIMESTAMPTZ,
    locked_by TEXT,
    last_error TEXT,
    correlation_id TEXT,
    organisation_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX jobs_claim_idx ON jobs (status, available_at, id)
    WHERE status IN ('PENDING', 'RETRY_SCHEDULED', 'RUNNING');
CREATE INDEX jobs_organisation_idx ON jobs (organisation_id, created_at) WHERE organisation_id IS NOT NULL;

CREATE TABLE outbox_events (
    id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    organisation_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    payload JSONB NOT NULL,
    correlation_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('PENDING', 'RUNNING', 'RETRY_SCHEDULED', 'DISPATCHED', 'DEAD')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL CHECK (max_attempts > 0),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at TIMESTAMPTZ,
    locked_by TEXT,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dispatched_at TIMESTAMPTZ
);

CREATE INDEX outbox_claim_idx ON outbox_events (status, available_at, id)
    WHERE status IN ('PENDING', 'RETRY_SCHEDULED', 'RUNNING');
CREATE INDEX outbox_aggregate_idx ON outbox_events (aggregate_type, aggregate_id, created_at);

CREATE TABLE idempotency_records (
    id UUID PRIMARY KEY,
    namespace TEXT NOT NULL,
    scope_key TEXT NOT NULL,
    organisation_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL,
    request_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PROCESSING', 'SUCCEEDED', 'FAILED')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    response JSONB,
    last_error TEXT,
    retry_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE (namespace, scope_key, idempotency_key)
);

CREATE INDEX idempotency_retry_idx ON idempotency_records (status, retry_at) WHERE status = 'FAILED';
