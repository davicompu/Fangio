-- Papers whose PDF text (or title/abstract) uses a glossary name. Keyed by the
-- normalised name (a term or one of its aliases) so merges carry mentions
-- along. Recomputed after glossary extraction and after building the index.
CREATE TABLE glossary_mention (
    name_key   TEXT NOT NULL,
    article_id INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    PRIMARY KEY (name_key, article_id)
);
