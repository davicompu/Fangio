-- AI transparency: one row per AI call, recording what actually ran.
CREATE TABLE ai_call (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    purpose         TEXT NOT NULL,
    model_requested TEXT NOT NULL,
    model_used      TEXT NOT NULL,
    prompt_hash     TEXT NOT NULL,
    temperature     REAL NOT NULL,
    seed            INTEGER,
    created_at      TEXT NOT NULL
);
