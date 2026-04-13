-- 0012_ai_screening.sql
-- AI screening: add per-article AI rationale and confidence score.
ALTER TABLE article ADD COLUMN ai_rationale TEXT;
ALTER TABLE article ADD COLUMN ai_confidence REAL;
