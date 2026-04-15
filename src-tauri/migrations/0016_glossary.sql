-- 0016_glossary.sql
-- Stores LLM-extracted domain terms for the project glossary.
-- Terms are extracted per-article and deduplicated by term text (case-insensitive).
CREATE TABLE IF NOT EXISTS glossary_term (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    term        TEXT NOT NULL,
    definition  TEXT NOT NULL,
    article_ids TEXT NOT NULL DEFAULT '[]',   -- JSON array of article IDs that use this term
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_glossary_term_lower ON glossary_term(lower(term));
