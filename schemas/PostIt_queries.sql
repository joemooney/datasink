-- PostIt Database Example Queries
-- These queries demonstrate how to work with the PostIt notes system

-- Get all open notes ordered by priority
SELECT id, title, description, priority, created_at
FROM notes
WHERE status = 'open'
ORDER BY 
    CASE priority
        WHEN 'critical' THEN 1
        WHEN 'high' THEN 2
        WHEN 'medium' THEN 3
        WHEN 'low' THEN 4
    END,
    created_at DESC;

-- Get all notes with their tags
SELECT 
    n.id,
    n.title,
    n.status,
    n.priority,
    GROUP_CONCAT(t.name) as tags
FROM notes n
LEFT JOIN note_tags nt ON n.id = nt.note_id
LEFT JOIN tags t ON nt.tag_id = t.id
GROUP BY n.id, n.title, n.status, n.priority;

-- Get notes by specific tag
SELECT n.*
FROM notes n
JOIN note_tags nt ON n.id = nt.note_id
JOIN tags t ON nt.tag_id = t.id
WHERE t.name = 'bug'
AND n.status != 'archived';

-- Get count of notes by status
SELECT 
    status,
    COUNT(*) as count
FROM notes
GROUP BY status;

-- Get high priority items that are not closed
SELECT * FROM notes
WHERE priority IN ('high', 'critical')
AND status NOT IN ('closed', 'archived')
ORDER BY 
    CASE priority
        WHEN 'critical' THEN 1
        WHEN 'high' THEN 2
    END,
    created_at;

-- Search notes by title or description
SELECT * FROM notes
WHERE (title LIKE '%meeting%' OR description LIKE '%meeting%')
AND status != 'archived';

-- Get notes created in the last 7 days
SELECT * FROM notes
WHERE created_at >= datetime('now', '-7 days')
ORDER BY created_at DESC;

-- Archive old closed notes (update example)
-- UPDATE notes 
-- SET status = 'archived'
-- WHERE status = 'closed'
-- AND created_at < datetime('now', '-30 days');

-- Add a tag to a note (insert example)
-- INSERT INTO note_tags (note_id, tag_id)
-- SELECT 1, id FROM tags WHERE name = 'urgent';

-- Get all tags with usage count
SELECT 
    t.id,
    t.name,
    t.color,
    COUNT(nt.note_id) as usage_count
FROM tags t
LEFT JOIN note_tags nt ON t.id = nt.tag_id
GROUP BY t.id, t.name, t.color
ORDER BY usage_count DESC;