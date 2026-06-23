-- 0025_pdf_source.sql
-- PDF provenance: how a PDF entered the workspace. NULL = manually added;
-- 'unpaywall' = auto-fetched open-access copy. source_url records the OA link.
-- The acquisition time is the existing pdf.created_at.
ALTER TABLE pdf ADD COLUMN source TEXT;
ALTER TABLE pdf ADD COLUMN source_url TEXT;
