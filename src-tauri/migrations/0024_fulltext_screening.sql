-- 0024_fulltext_screening.sql
-- Second screening pass (PRISMA full-text eligibility). fulltext_status marks an
-- article reviewed at full text: 'included' (kept) or 'excluded'. NULL = a
-- title/abstract-included article not yet full-text reviewed. Excluding at full
-- text also sets status='excluded' so it leaves the corpus via existing logic.
ALTER TABLE article ADD COLUMN fulltext_status TEXT;
