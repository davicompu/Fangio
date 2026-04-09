-- Fangio initial schema
-- All state for a single SLR project lives in one SQLite file inside the
-- project workspace folder. CSV/MD artifacts on disk are derived views.

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE project (
    id              INTEGER PRIMARY KEY CHECK (id = 1), -- single project per file
    name            TEXT NOT NULL,
    slr_type        TEXT,
    domain_subtype  TEXT,
    framework       TEXT,
    domain          TEXT,
    rq_what         TEXT,
    rq_how          TEXT,
    rq_where        TEXT,
    search_period_start TEXT,
    search_period_end   TEXT,
    language_filter TEXT,
    quality_list    TEXT,
    quality_threshold TEXT,
    onboarding_complete INTEGER NOT NULL DEFAULT 0,
    current_stage   TEXT NOT NULL DEFAULT 'onboarding',
    current_substage TEXT,
    schema_version  INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE search_query (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id      INTEGER NOT NULL DEFAULT 1 REFERENCES project(id),
    boolean_query   TEXT,
    core_keywords   TEXT,
    synonyms        TEXT,
    related_terms   TEXT,
    excluded_terms  TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE import_batch (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_database TEXT NOT NULL,
    format          TEXT NOT NULL,
    filename        TEXT NOT NULL,
    imported_at     TEXT NOT NULL,
    raw_record_count INTEGER NOT NULL DEFAULT 0,
    accepted_record_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE article (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    title           TEXT NOT NULL,
    authors         TEXT,
    year            INTEGER,
    journal         TEXT,
    abstract        TEXT,
    doi             TEXT,
    url             TEXT,
    source_databases TEXT,
    import_batches  TEXT,
    download_available TEXT,
    dedup_group_id  INTEGER,
    status          TEXT NOT NULL DEFAULT 'retrieved',
    exclusion_criterion INTEGER,
    exclusion_rationale TEXT,
    pdf_path        TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_article_status ON article(status);
CREATE INDEX idx_article_doi ON article(doi);
CREATE INDEX idx_article_dedup ON article(dedup_group_id);

CREATE TABLE coding (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id      INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    dimension       TEXT NOT NULL,
    code            TEXT NOT NULL,
    rationale       TEXT,
    source          TEXT NOT NULL DEFAULT 'researcher',
    model_name      TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_coding_article ON coding(article_id);

CREATE TABLE exclusion_criterion (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    number          INTEGER NOT NULL,
    label           TEXT NOT NULL,
    rationale       TEXT,
    is_deferred_quality_filter INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE pdf_chunk (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id      INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    page_start      INTEGER,
    page_end        INTEGER,
    chunk_index     INTEGER NOT NULL,
    text            TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE TABLE api_connector (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    enabled         INTEGER NOT NULL DEFAULT 0,
    base_url        TEXT NOT NULL,
    keychain_ref    TEXT,
    last_test_at    TEXT,
    last_test_ok    INTEGER
);

CREATE TABLE stage_report (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    stage           TEXT NOT NULL UNIQUE,
    status          TEXT NOT NULL DEFAULT 'not_started',
    body_md         TEXT,
    updated_at      TEXT NOT NULL
);

CREATE TABLE decision_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    stage           TEXT NOT NULL,
    sub_stage       TEXT,
    decision_key    TEXT NOT NULL,
    value_json      TEXT NOT NULL,
    rationale       TEXT,
    superseded_by   INTEGER REFERENCES decision_log(id),
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_decision_key ON decision_log(decision_key);
CREATE INDEX idx_decision_active ON decision_log(superseded_by) WHERE superseded_by IS NULL;

CREATE TABLE telemetry_event (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type      TEXT NOT NULL,
    payload_json    TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_telemetry_type ON telemetry_event(event_type);
CREATE INDEX idx_telemetry_created ON telemetry_event(created_at);

CREATE TABLE settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Seed defaults
INSERT INTO settings (key, value, updated_at) VALUES
    ('lmstudio.base_url',     '"http://127.0.0.1:1234/v1"', datetime('now')),
    ('lmstudio.chat_model',   '""',                          datetime('now')),
    ('lmstudio.embed_model',  '""',                          datetime('now')),
    ('telemetry.enabled',     'true',                        datetime('now')),
    ('telemetry.log_reasoning','false',                      datetime('now'));

INSERT INTO stage_report (stage, status, updated_at) VALUES
    ('assembling', 'not_started', datetime('now')),
    ('arranging',  'not_started', datetime('now')),
    ('assessing',  'not_started', datetime('now'));
