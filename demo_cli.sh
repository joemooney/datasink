#!/bin/bash

echo "DataSink CLI Demo"
echo "================="
echo ""

# Build the project
echo "Building the project..."
cargo build --release

# Path to the binary
DATASINK="./target/release/datasink"

echo ""
echo "1. Creating a new database..."
$DATASINK server create-database demo.db

echo ""
echo "2. Starting the server in the background..."
$DATASINK server start -d sqlite://demo.db &
SERVER_PID=$!
sleep 2

echo ""
echo "3. Creating a users table..."
$DATASINK server create-table users '[
  {"name":"id","type":"INTEGER","primary_key":true},
  {"name":"name","type":"TEXT","nullable":false},
  {"name":"email","type":"TEXT","unique":true},
  {"name":"created_at","type":"TIMESTAMP"}
]'

echo ""
echo "4. Inserting some users..."
$DATASINK insert users '{"id":1,"name":"Alice Smith","email":"alice@example.com","created_at":1700000000}'
$DATASINK insert users '{"id":2,"name":"Bob Jones","email":"bob@example.com","created_at":1700000100}'
$DATASINK insert users '{"id":3,"name":"Carol Davis","email":"carol@example.com","created_at":1700000200}'

echo ""
echo "5. Querying all users (table format)..."
$DATASINK query "SELECT * FROM users ORDER BY id"

echo ""
echo "6. Querying users in JSON format..."
$DATASINK query "SELECT * FROM users WHERE id < 3" -f json

echo ""
echo "7. Updating a user..."
$DATASINK update users '{"email":"alice.smith@example.com"}' -w "id = 1"

echo ""
echo "8. Querying to see the update..."
$DATASINK query "SELECT * FROM users WHERE id = 1"

echo ""
echo "9. Deleting a user..."
$DATASINK delete users -w "id = 3"

echo ""
echo "10. Final query to see all remaining users..."
$DATASINK query "SELECT * FROM users" -f csv

echo ""
echo "Stopping the server..."
kill $SERVER_PID 2>/dev/null

echo ""
echo "Demo complete!"