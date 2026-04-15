-- 0015_ai_coding.sql
-- AI coding: stores per-rater code suggestions pending human resolution.
-- When both raters agree on a code it is applied directly to 'coding';
-- disagreements land here until the researcher accepts or rejects them.
CREATE TABLE ai_coding_proposal (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    dimension  TEXT NOT NULL,
    code       TEXT NOT NULL,
    rationale  TEXT,
    rater      INTEGER NOT NULL CHECK(rater IN (1, 2)),
    model_name TEXT,
    run_id     TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_acp_article ON ai_coding_proposal(article_id);
