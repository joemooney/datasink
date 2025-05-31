-- Create notes_history table with composite primary key
CREATE TABLE IF NOT EXISTS notes_history (
    id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL,
    created_by TEXT,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    url TEXT,
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by TEXT,
    operation TEXT NOT NULL,
    PRIMARY KEY (id, last_updated)
);

-- Create index for efficient queries by note id
CREATE INDEX IF NOT EXISTS idx_notes_history_id ON notes_history(id);

-- Create index for efficient queries by timestamp
CREATE INDEX IF NOT EXISTS idx_notes_history_updated ON notes_history(last_updated);