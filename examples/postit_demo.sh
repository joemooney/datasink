#!/bin/bash

# PostIt Notes Demo Script
# This script demonstrates using DataSink with the PostIt schema

echo "PostIt Notes Demo"
echo "================="
echo ""

# Ensure we're in the correct directory
cd "$(dirname "$0")/.." || exit 1

# Build the project if needed
echo "Building DataSink..."
cargo build --release --quiet

# Set the binary path
DATASINK="./target/release/datasink"

# Create the PostIt database from schema
echo "Creating PostIt database from schema..."
$DATASINK server create-from-schema schemas/PostIt.schema

echo ""
echo "Starting DataSink server with PostIt database..."
$DATASINK server start -d sqlite://postit.db &
SERVER_PID=$!
sleep 2

echo ""
echo "=== Querying all notes ==="
$DATASINK query "SELECT id, title, status, priority FROM notes ORDER BY id"

echo ""
echo "=== High priority open items ==="
$DATASINK query "SELECT title, description, priority FROM notes WHERE priority IN ('high', 'critical') AND status = 'open'"

echo ""
echo "=== Notes with tags (JSON format) ==="
$DATASINK query "SELECT n.id, n.title, n.status, GROUP_CONCAT(t.name) as tags FROM notes n LEFT JOIN note_tags nt ON n.id = nt.note_id LEFT JOIN tags t ON nt.tag_id = t.id GROUP BY n.id" -f json

echo ""
echo "=== Adding a new note ==="
$DATASINK insert notes '{
  "title": "Review pull requests",
  "description": "Check and merge pending PRs from the team",
  "status": "open",
  "priority": "high",
  "url": "https://github.com/myorg/myrepo/pulls"
}'

echo ""
echo "=== Updating note status ==="
$DATASINK update notes '{"status": "working"}' -w "title = 'Setup database backups'"

echo ""
echo "=== All available tags ==="
$DATASINK query "SELECT name, color FROM tags ORDER BY name" -f csv

echo ""
echo "Stopping server..."
kill $SERVER_PID 2>/dev/null

echo ""
echo "Demo complete! The PostIt database is available at: postit.db"
echo ""
echo "To start using it:"
echo "  1. Start server: datasink server start -d sqlite://postit.db"
echo "  2. Query notes:  datasink query 'SELECT * FROM notes'"
echo "  3. Add notes:    datasink insert notes '{\"title\": \"My note\", \"priority\": \"medium\"}'"