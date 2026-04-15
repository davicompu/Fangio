-- Set the default chat model for projects that have never configured one.
UPDATE settings
SET value = '"google/gemma-4-e2b"', updated_at = datetime('now')
WHERE key = 'lmstudio.chat_model' AND (value = '""' OR value = '' OR value IS NULL);
