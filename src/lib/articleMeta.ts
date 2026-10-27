// Helpers for the JSON-encoded TEXT columns on articles / glossary terms.
// These columns store small JSON arrays as strings; parse defensively so a
// malformed value never throws into a render.

/** Parse an article's `source_databases` column into a display list. Accepts a
 *  JSON array, a bare string, or null. */
export function parseSources(raw: string | null | undefined): string[] {
  if (!raw) return [];
  try {
    const v = JSON.parse(raw);
    if (Array.isArray(v)) return v.filter((x) => typeof x === "string");
    if (typeof v === "string") return [v];
  } catch {
    return [raw];
  }
  return [];
}

/** Parse a glossary term's `article_ids` JSON string into numeric ids. */
export function parseArticleIds(raw: string | null | undefined): number[] {
  if (!raw) return [];
  try {
    const v = JSON.parse(raw);
    return Array.isArray(v) ? v.filter((x) => typeof x === "number") : [];
  } catch {
    return [];
  }
}

/** Parse a glossary term's `aliases` JSON string into a list. */
export function parseAliases(raw: string | null | undefined): string[] {
  if (!raw) return [];
  try {
    const v = JSON.parse(raw);
    return Array.isArray(v) ? v.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

/** Every paper linked to a glossary term: the papers it was extracted from
 *  plus the papers whose text mentions it. */
export function termPaperIds(t: { article_ids: string; mention_ids?: string }): number[] {
  return [...new Set([...parseArticleIds(t.article_ids), ...parseArticleIds(t.mention_ids)])];
}
