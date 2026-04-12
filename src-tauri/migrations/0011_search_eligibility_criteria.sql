-- Search eligibility criteria: stored as JSON arrays of plain-text strings.
-- These are the inclusion / exclusion rules that guide which retrieved records
-- qualify for the review (distinct from the arranging-stage exclusion criteria
-- applied during title & abstract screening).

ALTER TABLE project ADD COLUMN search_inclusion_criteria TEXT;
ALTER TABLE project ADD COLUMN search_exclusion_criteria TEXT;
