-- "Opened" events for AI outputs other than screening decisions: coding
-- suggestions (target = "article_id|dimension|code", so the history survives
-- accepting the suggestion), and the codebook, synthesis and agenda outputs
-- (target = the decision-log id of the AI/app entry).
CREATE TABLE ai_output_open (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    kind      TEXT NOT NULL,     -- coding | codebook | synthesis | agenda
    target    TEXT NOT NULL,
    opened_by TEXT NOT NULL,
    opened_at TEXT NOT NULL
);
CREATE INDEX idx_aoo_target ON ai_output_open(kind, target);
