-- Who produced each decision-log entry: 'human' (the researcher) or 'ai' (an AI
-- assistant, e.g. the setup assistant's clarifying questions and suggestions).
-- Every earlier entry was written by the researcher.
ALTER TABLE decision_log ADD COLUMN actor TEXT NOT NULL DEFAULT 'human';
