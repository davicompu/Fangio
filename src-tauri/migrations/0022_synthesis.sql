-- 0022_synthesis.sql
-- Stage 3 Evaluation: the generated synthesis narrative (organised by framework
-- dimension, cited). Feeds the manuscript's Results section.
ALTER TABLE project ADD COLUMN synthesis TEXT;
