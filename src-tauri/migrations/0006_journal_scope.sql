-- Journal scope list: seeded with high-impact business journals.
-- abs_rating: '4*', '4', '3', '2', '1'  (ABS Academic Journal Guide)
-- abdc_rating: 'A*', 'A', 'B', 'C'      (ABDC Australian Business Deans Council)
-- impact_factor: approximate, may lag one year

CREATE TABLE IF NOT EXISTS journal_scope (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT    NOT NULL,
    issn          TEXT,
    abs_rating    TEXT,
    abdc_rating   TEXT,
    impact_factor REAL,
    field         TEXT,
    is_custom     INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO journal_scope (name, issn, abs_rating, abdc_rating, impact_factor, field, is_custom) VALUES
-- ── Management / Strategy ──────────────────────────────────────────────
('Academy of Management Journal',                '0001-4273', '4*', 'A*', 9.0,  'Management', 0),
('Academy of Management Review',                 '0363-7425', '4*', 'A*', 12.0, 'Management', 0),
('Administrative Science Quarterly',             '0001-8392', '4*', 'A*', 10.0, 'Management', 0),
('Journal of Management',                        '0149-2063', '4*', 'A*', 9.0,  'Management', 0),
('Organization Science',                         '1047-7039', '4*', 'A*', 5.5,  'Management', 0),
('Strategic Management Journal',                 '0143-2095', '4*', 'A*', 8.0,  'Management', 0),
('Management Science',                           '0025-1909', '4*', 'A*', 5.5,  'Management', 0),
('Journal of Management Studies',                '0022-2380', '4',  'A*', 6.5,  'Management', 0),
('Organization Studies',                         '0170-8406', '4',  'A*', 5.5,  'Management', 0),
('Journal of Organizational Behavior',           '0894-3796', '4',  'A*', 6.5,  'Management', 0),
('Organizational Behavior and Human Decision Processes', '0749-5978', '4', 'A*', 4.5, 'Management', 0),
('British Journal of Management',                '1045-3172', '3',  'A',  4.0,  'Management', 0),
('Long Range Planning',                          '0024-6301', '3',  'A',  6.0,  'Management', 0),
('California Management Review',                 '0008-1256', '3',  'A*', 5.0,  'Management', 0),
('Harvard Business Review',                      '0017-8012', '3',  'A*', 6.0,  'Management', 0),

-- ── Finance & Accounting ───────────────────────────────────────────────
('Journal of Finance',                           '0022-1082', '4*', 'A*', 8.5,  'Finance', 0),
('Journal of Financial Economics',               '0304-405X', '4*', 'A*', 9.0,  'Finance', 0),
('Review of Financial Studies',                  '0893-9454', '4*', 'A*', 7.5,  'Finance', 0),
('Journal of Accounting and Economics',          '0165-4101', '4*', 'A*', 7.0,  'Accounting', 0),
('Journal of Accounting Research',               '0021-8456', '4*', 'A*', 5.5,  'Accounting', 0),
('The Accounting Review',                        '0001-4826', '4*', 'A*', 5.0,  'Accounting', 0),
('Accounting, Organizations and Society',        '0361-3682', '4*', 'A*', 5.5,  'Accounting', 0),
('Journal of Financial and Quantitative Analysis','0022-1090','4',  'A*', 4.5,  'Finance', 0),
('Contemporary Accounting Research',             '0823-9150', '4',  'A*', 4.0,  'Accounting', 0),
('Review of Accounting Studies',                 '1380-6653', '3',  'A',  4.0,  'Accounting', 0),
('European Accounting Review',                   '0963-8180', '3',  'A',  3.0,  'Accounting', 0),

-- ── Marketing ─────────────────────────────────────────────────────────
('Journal of Marketing',                         '0022-2429', '4*', 'A*', 12.0, 'Marketing', 0),
('Journal of Marketing Research',                '0022-2437', '4*', 'A*', 6.5,  'Marketing', 0),
('Journal of Consumer Research',                 '0093-5301', '4*', 'A*', 8.0,  'Marketing', 0),
('Marketing Science',                            '0732-2399', '4*', 'A*', 4.5,  'Marketing', 0),
('Journal of the Academy of Marketing Science',  '0092-0703', '4',  'A*', 11.0, 'Marketing', 0),
('International Journal of Research in Marketing','0167-8116','3',  'A',  5.0,  'Marketing', 0),
('Journal of Retailing',                         '0022-4359', '3',  'A',  9.0,  'Marketing', 0),
('European Journal of Marketing',                '0309-0566', '2',  'A',  4.0,  'Marketing', 0),

-- ── Information Systems ────────────────────────────────────────────────
('MIS Quarterly',                                '0276-7783', '4*', 'A*', 7.5,  'Information Systems', 0),
('Information Systems Research',                 '1047-7047', '4*', 'A*', 6.0,  'Information Systems', 0),
('Journal of Management Information Systems',    '0742-1222', '3',  'A',  6.5,  'Information Systems', 0),
('European Journal of Information Systems',      '0960-085X', '3',  'A',  5.5,  'Information Systems', 0),
('Information & Management',                     '0378-7206', '3',  'A',  8.0,  'Information Systems', 0),
('Journal of Information Technology',            '0268-3962', '3',  'A*', 9.0,  'Information Systems', 0),
('Decision Support Systems',                     '0167-9236', '2',  'A',  6.0,  'Information Systems', 0),
('Computers in Human Behavior',                  '0747-5632', '2',  'A',  9.0,  'Information Systems', 0),
('Internet Research',                            '1066-2243', '2',  'A',  7.0,  'Information Systems', 0),

-- ── Operations & Supply Chain ──────────────────────────────────────────
('Operations Research',                          '0030-364X', '4*', 'A*', 5.0,  'Operations', 0),
('Journal of Operations Management',             '0272-6963', '4*', 'A*', 8.0,  'Operations', 0),
('Production and Operations Management',         '1059-1478', '4',  'A*', 5.0,  'Operations', 0),
('International Journal of Operations & Production Management', '0144-3577', '3', 'A', 6.0, 'Operations', 0),
('Supply Chain Management: An International Journal', '1359-8546', '3', 'A', 8.0, 'Operations', 0),
('Journal of Supply Chain Management',           '1523-2409', '3',  'A',  8.5,  'Operations', 0),

-- ── Human Resource Management / OB ────────────────────────────────────
('Journal of Applied Psychology',                '0021-9010', '4*', 'A*', 12.0, 'HRM / OB', 0),
('Personnel Psychology',                         '0031-5826', '4*', 'A*', 8.5,  'HRM / OB', 0),
('Human Resource Management',                    '0090-4848', '3',  'A*', 6.0,  'HRM / OB', 0),
('Human Resource Management Journal',            '0954-5395', '3',  'A*', 5.5,  'HRM / OB', 0),
('International Journal of Human Resource Management', '0958-5192', '3', 'A', 5.0, 'HRM / OB', 0),
('Work, Employment and Society',                 '0950-0170', '3',  'A',  4.5,  'HRM / OB', 0),

-- ── Entrepreneurship & Innovation ─────────────────────────────────────
('Journal of Business Venturing',                '0883-9026', '4*', 'A*', 9.0,  'Entrepreneurship', 0),
('Entrepreneurship Theory and Practice',         '1042-2587', '4',  'A*', 8.0,  'Entrepreneurship', 0),
('Strategic Entrepreneurship Journal',           '1932-4391', '4',  'A*', 7.0,  'Entrepreneurship', 0),
('Small Business Economics',                     '0921-898X', '3',  'A',  7.5,  'Entrepreneurship', 0),
('Research Policy',                              '0048-7333', '3',  'A*', 9.0,  'Innovation', 0),
('Journal of Product Innovation Management',     '0737-6782', '3',  'A',  6.0,  'Innovation', 0),

-- ── International Business ─────────────────────────────────────────────
('Journal of International Business Studies',    '0047-2506', '4*', 'A*', 9.0,  'International Business', 0),
('Journal of World Business',                    '1090-9516', '3',  'A*', 9.0,  'International Business', 0),
('International Business Review',                '0969-5931', '3',  'A',  6.0,  'International Business', 0),
('Management International Review',              '0938-8249', '2',  'A',  4.5,  'International Business', 0);
