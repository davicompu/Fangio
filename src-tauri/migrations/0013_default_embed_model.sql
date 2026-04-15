-- Set default models for projects that have never configured them.
UPDATE settings
SET value = '"text-embedding-embeddinggemma-300m-qat"', updated_at = datetime('now')
WHERE key = 'lmstudio.embed_model' AND (value = '""' OR value = '' OR value IS NULL);

UPDATE settings
SET value = '"google/gemma-4-e2b"', updated_at = datetime('now')
WHERE key = 'lmstudio.chat_model' AND (value = '""' OR value = '' OR value IS NULL);
