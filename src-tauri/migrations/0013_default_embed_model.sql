-- Set the default embedding model for projects that have never configured one.
UPDATE settings
SET value = '"text-embedding-embeddinggemma-300m-qat"', updated_at = datetime('now')
WHERE key = 'lmstudio.embed_model' AND (value = '""' OR value = '' OR value IS NULL);
