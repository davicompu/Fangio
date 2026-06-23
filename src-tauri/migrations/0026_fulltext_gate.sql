-- 0026_fulltext_gate.sql
-- Full-text screening becomes a required PRISMA gate: only articles kept at full
-- text are coded. Backfill: articles that were already coded under the old
-- (lenient) model are treated as full-text-included so existing work is kept.
-- Uncoded title/abstract-included articles must now be reviewed at full text.
UPDATE article SET fulltext_status = 'included'
WHERE status = 'included'
  AND fulltext_status IS NULL
  AND id IN (SELECT DISTINCT article_id FROM coding);
