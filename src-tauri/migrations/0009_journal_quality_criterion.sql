-- Stores the minimum journal quality thresholds as a JSON blob.
-- Schema: { "abs_min": "2", "abdc_min": "B" }  (null = no threshold set)
ALTER TABLE project ADD COLUMN journal_quality TEXT NULL;
