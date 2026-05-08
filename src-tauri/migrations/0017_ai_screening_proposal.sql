CREATE TABLE ai_screening_proposal (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    rater INTEGER NOT NULL CHECK (rater IN (1, 2)),
    decision TEXT NOT NULL,
    criterion_number INTEGER,
    rationale TEXT NOT NULL,
    confidence REAL NOT NULL,
    model_name TEXT,
    run_id TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_asp_article ON ai_screening_proposal(article_id);
CREATE INDEX idx_asp_rater ON ai_screening_proposal(rater);
