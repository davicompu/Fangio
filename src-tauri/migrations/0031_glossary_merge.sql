-- Glossary merges by meaning. Suggestions are computed on demand from term
-- embeddings; only the researcher's decisions are stored, keyed by normalised
-- term names so they survive re-extracting the glossary.
CREATE TABLE glossary_term_embedding (
    term_key        TEXT NOT NULL,
    embedding_model TEXT NOT NULL,
    embedding       BLOB NOT NULL,
    PRIMARY KEY (term_key, embedding_model)
);

CREATE TABLE glossary_merge (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    key_a           TEXT NOT NULL,          -- normalised names, key_a < key_b
    key_b           TEXT NOT NULL,
    term_a          TEXT NOT NULL,          -- names as shown when decided
    term_b          TEXT NOT NULL,
    similarity      REAL,
    embedding_model TEXT,
    status          TEXT NOT NULL,          -- accepted | dismissed | undone
    keep_term       TEXT,                   -- name kept when accepted
    snapshot        TEXT,                   -- both rows before the merge (JSON), for undo
    decided_at      TEXT NOT NULL,
    UNIQUE (key_a, key_b)
);
