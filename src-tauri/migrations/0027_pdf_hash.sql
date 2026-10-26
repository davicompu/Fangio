-- Content hash (SHA-256, hex) of each PDF's bytes, so we can detect when the
-- same file is uploaded again — whether to the same article or a different one.
-- Existing rows are backfilled lazily (their stored files are hashed on demand).
ALTER TABLE pdf ADD COLUMN content_hash TEXT;
CREATE INDEX idx_pdf_content_hash ON pdf(content_hash);
