-- Arranging stage additions: predefined framework codes, exclusion criteria
-- seeding, and tracking which articles have been screened.

CREATE TABLE framework_code_definition (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    dimension       TEXT NOT NULL,
    code            TEXT NOT NULL,
    description     TEXT,
    created_at      TEXT NOT NULL,
    UNIQUE(dimension, code)
);

CREATE INDEX idx_fcd_dim ON framework_code_definition(dimension);

-- Track that an article has been seen by the screener (even if no exclusion
-- was applied). Lets us tell "screened and kept" from "not yet screened".
ALTER TABLE article ADD COLUMN screened_at TEXT;

-- Seed the standard SPAR-4-SLR exclusion criteria. Researcher can edit/delete.
INSERT INTO exclusion_criterion (number, label, rationale, is_deferred_quality_filter) VALUES
    (1, 'Duplicate', 'Already present elsewhere in the corpus', 0),
    (2, 'Outside search period', 'Publication year outside the configured search window', 0),
    (3, 'Below source quality threshold', 'Source ranks below the configured quality list threshold', 0),
    (4, 'Wrong source type', 'Not a journal article (or other source type the review excludes)', 0),
    (5, 'Non-English', 'Language outside the configured language filter', 0),
    (6, 'Predatory journal', 'Published in a journal flagged as predatory', 0),
    (7, 'Full text not accessible', 'No way to retrieve the article''s full text', 0),
    (8, 'Not directly relevant to domain', 'Title and abstract suggest the article is off-topic', 0),
    (9, 'Retracted', 'Article has been retracted by the publisher', 0);
