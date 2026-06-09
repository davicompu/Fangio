-- 0018_glossary_aliases.sql
-- Tracks alternate spellings / abbreviations for a glossary term so that
-- "Critical Thinking" and "Critical Thinking (CT)" collapse into one entry.
ALTER TABLE glossary_term ADD COLUMN aliases TEXT NOT NULL DEFAULT '[]';
