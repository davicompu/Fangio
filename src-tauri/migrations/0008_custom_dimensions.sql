-- Stores custom framework dimension names as a JSON array of strings.
-- Only used when project.framework = 'Custom'.
ALTER TABLE project ADD COLUMN custom_dimensions TEXT NULL;
