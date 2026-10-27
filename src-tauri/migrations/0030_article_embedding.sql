-- Embeddings of each article's title + abstract, for meaning-based search over
-- papers that have no PDF (and alongside PDF chunks for those that do).
-- text_hash detects when the title/abstract changed; embedding_model detects a
-- change of embedding model. Either one triggers a re-embed.
CREATE TABLE article_embedding (
    article_id      INTEGER PRIMARY KEY REFERENCES article(id) ON DELETE CASCADE,
    text_hash       TEXT NOT NULL,
    embedding       BLOB NOT NULL,
    embedding_model TEXT NOT NULL,
    created_at      TEXT NOT NULL
);
