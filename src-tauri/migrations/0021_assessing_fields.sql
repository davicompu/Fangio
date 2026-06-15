-- 0021_assessing_fields.sql
-- Stage 3 (Assessing) deliverables per SPAR-4-SLR: the analysis method and
-- research agenda (Evaluation 3a), and the review limitations and sources of
-- support / funding (Reporting 3b). All feed the generated manuscript.
ALTER TABLE project ADD COLUMN analysis_method TEXT;
ALTER TABLE project ADD COLUMN research_agenda TEXT;
ALTER TABLE project ADD COLUMN review_limitations TEXT;
ALTER TABLE project ADD COLUMN sources_of_support TEXT;
