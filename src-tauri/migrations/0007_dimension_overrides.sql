-- Stores per-project dimension overrides as a JSON blob.
-- Schema: { "hidden": ["DimName"], "renamed": { "OldName": "NewName" } }
ALTER TABLE project ADD COLUMN dimension_overrides TEXT NULL;
