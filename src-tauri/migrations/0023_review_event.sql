-- 0023_review_event.sql
-- Phase 2 of the decision report: a forward log of review actions that were not
-- previously captured — AI-proposal rejects and deduplication merges — so the
-- decision report reflects them. Each row records who acted (human/ai).
CREATE TABLE review_event (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    stage      TEXT NOT NULL,
    action     TEXT NOT NULL,
    target     TEXT,
    detail     TEXT,
    rationale  TEXT,
    actor      TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_review_event_created ON review_event(created_at);
