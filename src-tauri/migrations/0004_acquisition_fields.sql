-- Add boolean_query and databases columns to project so the acquisition form
-- persists its state across navigation (previously only written to decision_log).
ALTER TABLE project ADD COLUMN boolean_query TEXT;
ALTER TABLE project ADD COLUMN databases TEXT;
