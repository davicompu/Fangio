-- 0020_ai_consolidation.sql
-- Corpus-level code consolidation (Phase 4 of AI coding). Records each
-- variant→canonical merge so the modal can show "(was: …)" and offer Undo.
-- `coding.consolidated_from` holds the original label a row was renamed from.
ALTER TABLE coding ADD COLUMN consolidated_from TEXT;

CREATE TABLE ai_consolidation (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    dimension      TEXT NOT NULL,
    canonical_code TEXT NOT NULL,
    variant_code   TEXT NOT NULL,
    count          INTEGER NOT NULL,
    run_id         TEXT NOT NULL,
    created_at     TEXT NOT NULL
);
CREATE INDEX idx_consol_dim ON ai_consolidation(dimension);
