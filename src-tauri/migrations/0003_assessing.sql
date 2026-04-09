-- Assessing stage: PDFs, embeddings, RAG.
--
-- pdf_chunk already exists from 0001 (with article_id NOT NULL). We add pdf_id
-- + embedding columns; chunks are only created for PDFs that have been linked
-- to an article, so the original NOT NULL invariant still holds.

CREATE TABLE pdf (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    filename        TEXT NOT NULL,
    stored_path     TEXT NOT NULL,        -- path inside <workspace>/pdfs/
    source_path     TEXT NOT NULL,        -- original path picked by user
    page_count      INTEGER,
    text_length     INTEGER,
    article_id      INTEGER REFERENCES article(id),
    indexed_at      TEXT,                 -- when text was extracted + chunked
    embedded_at     TEXT,                 -- when embeddings were computed
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_pdf_article ON pdf(article_id);

ALTER TABLE pdf_chunk ADD COLUMN pdf_id INTEGER REFERENCES pdf(id);
ALTER TABLE pdf_chunk ADD COLUMN embedding BLOB;
ALTER TABLE pdf_chunk ADD COLUMN embedding_model TEXT;

CREATE INDEX idx_pdf_chunk_pdf ON pdf_chunk(pdf_id);
