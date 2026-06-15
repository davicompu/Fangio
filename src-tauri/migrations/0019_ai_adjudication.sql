-- 0019_ai_adjudication.sql
-- Records each disagreement pair an independent AI adjudicator merged into a
-- single applied code. Powers the "Resolved by AI adjudicator" review section
-- and the Undo action (which restores both original proposals).
CREATE TABLE ai_adjudication (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id            INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    dimension             TEXT NOT NULL,
    canonical_code        TEXT NOT NULL,
    coding_id             INTEGER,
    label1                TEXT NOT NULL,
    rater1                INTEGER NOT NULL,
    rationale1            TEXT,
    model1                TEXT,
    label2                TEXT NOT NULL,
    rater2                INTEGER NOT NULL,
    rationale2            TEXT,
    model2                TEXT,
    adjudicator_rationale TEXT,
    run_id                TEXT NOT NULL,
    created_at            TEXT NOT NULL
);
CREATE INDEX idx_adj_article ON ai_adjudication(article_id);
