-- Script to ensure notes, notes_archive, and notes_history tables stay in sync
-- Run this after any ALTER TABLE commands on the notes table

-- Example: If you add a column to notes:
-- ALTER TABLE notes ADD COLUMN new_field TEXT;
-- Then run these to keep the other tables in sync:
-- ALTER TABLE notes_archive ADD COLUMN new_field TEXT;
-- ALTER TABLE notes_history ADD COLUMN new_field TEXT;

-- To view the current schema of all three tables:
SELECT sql FROM sqlite_master 
WHERE type='table' 
AND name IN ('notes', 'notes_archive', 'notes_history')
ORDER BY name;

-- To check if all tables have the same columns (except the extra columns):
-- This query shows columns that exist in notes but not in notes_archive
SELECT 'notes_archive missing: ' || name AS issue
FROM pragma_table_info('notes') 
WHERE name NOT IN (
    SELECT name FROM pragma_table_info('notes_archive')
)
AND name NOT IN ('deleted_at', 'deleted_by')

UNION ALL

-- This query shows columns that exist in notes but not in notes_history
SELECT 'notes_history missing: ' || name AS issue
FROM pragma_table_info('notes') 
WHERE name NOT IN (
    SELECT name FROM pragma_table_info('notes_history')
)
AND name NOT IN ('last_updated', 'updated_by', 'operation')

UNION ALL

-- This query shows columns that exist in notes_archive but not in notes
SELECT 'notes missing from archive: ' || name AS issue
FROM pragma_table_info('notes_archive') 
WHERE name NOT IN (
    SELECT name FROM pragma_table_info('notes')
)
AND name NOT IN ('deleted_at', 'deleted_by')

UNION ALL

-- This query shows columns that exist in notes_history but not in notes
SELECT 'notes missing from history: ' || name AS issue
FROM pragma_table_info('notes_history') 
WHERE name NOT IN (
    SELECT name FROM pragma_table_info('notes')
)
AND name NOT IN ('last_updated', 'updated_by', 'operation');