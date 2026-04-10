-- Seed database access URLs. Google Scholar is free so it gets a default.
-- All others are left empty; users fill in their institutional access URLs.
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('database.scopus.url',        '""', datetime('now')),
    ('database.wos.url',           '""', datetime('now')),
    ('database.ebsco-bsc.url',     '""', datetime('now')),
    ('database.pubmed.url',        '""', datetime('now')),
    ('database.ieee.url',          '""', datetime('now')),
    ('database.acm.url',           '""', datetime('now')),
    ('database.scholar.url',       '"https://scholar.google.com"', datetime('now')),
    ('database.sciencedirect.url', '""', datetime('now'));
