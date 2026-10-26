-- AI transparency: one row per AI call, recording what actually ran.
CREATE TABLE ai_call (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    purpose         TEXT NOT NULL,
    model_requested TEXT NOT NULL,
    model_used      TEXT NOT NULL,
    prompt_hash     TEXT NOT NULL,
    temperature     REAL NOT NULL,
    seed            INTEGER,
    created_at      TEXT NOT NULL
);

-- Rebuild ai_screening_proposal as the full screening log (confidence may be NULL).
-- auto_applied is backfilled for old rows whose decision is still the one on the article.
CREATE TABLE ai_screening_proposal_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    rater INTEGER NOT NULL DEFAULT 1,
    stage TEXT NOT NULL DEFAULT 'title_abstract',   -- title_abstract | fulltext
    mode TEXT NOT NULL DEFAULT 'screening',          -- screening | purification
    decision TEXT NOT NULL,                          -- include|exclude|uncertain|error|skipped
    criterion_number INTEGER,
    rationale TEXT NOT NULL,
    confidence REAL,
    model_name TEXT,
    ai_call_id INTEGER REFERENCES ai_call(id),
    auto_applied INTEGER NOT NULL DEFAULT 0,
    overridden_at TEXT,
    overridden_to TEXT,                               -- include|exclude|reverted|reset
    run_id TEXT NOT NULL,
    created_at TEXT NOT NULL
);
INSERT INTO ai_screening_proposal_new (id, article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at, auto_applied)
SELECT p.id, p.article_id, p.rater, p.decision, p.criterion_number, p.rationale, p.confidence, p.model_name, p.run_id, p.created_at,
       CASE WHEN a.ai_rationale = p.rationale AND
                 ((p.decision='exclude' AND a.status='excluded') OR (p.decision='include' AND a.status='included'))
            THEN 1 ELSE 0 END
FROM ai_screening_proposal p JOIN article a ON a.id = p.article_id;
DROP TABLE ai_screening_proposal;
ALTER TABLE ai_screening_proposal_new RENAME TO ai_screening_proposal;
CREATE INDEX idx_asp_article ON ai_screening_proposal(article_id, stage);

CREATE TABLE ai_screening_open (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id INTEGER NOT NULL REFERENCES ai_screening_proposal(id) ON DELETE CASCADE,
    opened_by TEXT NOT NULL,
    via TEXT NOT NULL,          -- detail | decision
    opened_at TEXT NOT NULL
);
CREATE INDEX idx_aso_proposal ON ai_screening_open(proposal_id);

-- Task 7: coding transparency
ALTER TABLE coding ADD COLUMN rationale2 TEXT;
ALTER TABLE coding ADD COLUMN model_name2 TEXT;
ALTER TABLE coding ADD COLUMN confidence REAL;
ALTER TABLE coding ADD COLUMN confidence2 REAL;
ALTER TABLE coding ADD COLUMN ai_call_id INTEGER;
ALTER TABLE coding ADD COLUMN ai_call_id2 INTEGER;
ALTER TABLE ai_coding_proposal ADD COLUMN agreement TEXT NOT NULL DEFAULT 'one';
ALTER TABLE ai_coding_proposal ADD COLUMN confidence REAL;
ALTER TABLE ai_coding_proposal ADD COLUMN ai_call_id INTEGER;
ALTER TABLE ai_coding_proposal ADD COLUMN rationale2 TEXT;
ALTER TABLE ai_coding_proposal ADD COLUMN model_name2 TEXT;
ALTER TABLE ai_coding_proposal ADD COLUMN confidence2 REAL;
ALTER TABLE ai_coding_proposal ADD COLUMN ai_call_id2 INTEGER;
-- Existing projects keep today's behaviour; a brand-new DB has no project row yet at migration time.
INSERT OR IGNORE INTO settings (key, value, updated_at)
SELECT 'coding.agreed_codes_mode', '"auto"', datetime('now') WHERE EXISTS (SELECT 1 FROM project WHERE id = 1);
