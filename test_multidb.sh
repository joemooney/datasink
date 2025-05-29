#!/bin/bash
# Test script for multi-database functionality

echo "🚀 Testing Multi-Database Support in DataSink"
echo "============================================"

# Clean up any existing test databases
rm -f test_main.db test_analytics.db test_logs.db

# Start the server in the background
echo "Starting server with main database..."
./target/debug/datasink server start -d sqlite://test_main.db -b 127.0.0.1:50053 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Check server status
echo -e "\n📊 Initial server status:"
./target/debug/datasink server status -s http://127.0.0.1:50053

# Add additional databases
echo -e "\n➕ Adding analytics database..."
./target/debug/datasink server add-database analytics sqlite://test_analytics.db -s http://127.0.0.1:50053

echo -e "\n➕ Adding logs database..."
./target/debug/datasink server add-database logs sqlite://test_logs.db -s http://127.0.0.1:50053

# Check status again
echo -e "\n📊 Server status after adding databases:"
./target/debug/datasink server status -s http://127.0.0.1:50053

# Create tables in different databases
echo -e "\n📋 Creating tables in different databases..."

# Create table in default database
echo "Creating 'users' table in default database..."
./target/debug/datasink server create-table users '[{"name":"id","type":"INTEGER","primary_key":true},{"name":"name","type":"TEXT"},{"name":"email","type":"TEXT"}]' -s http://127.0.0.1:50053

# Create table in analytics database
echo "Creating 'events' table in analytics database..."
./target/debug/datasink query "CREATE TABLE events (id INTEGER PRIMARY KEY, event_type TEXT, timestamp INTEGER)" -D analytics -s http://127.0.0.1:50053

# Create table in logs database
echo "Creating 'access_logs' table in logs database..."
./target/debug/datasink query "CREATE TABLE access_logs (id INTEGER PRIMARY KEY, ip TEXT, path TEXT, timestamp INTEGER)" -D logs -s http://127.0.0.1:50053

# Insert data into different databases
echo -e "\n➕ Inserting data into different databases..."

# Insert into default database
./target/debug/datasink insert users '{"id": 1, "name": "Alice", "email": "alice@example.com"}' -s http://127.0.0.1:50053
./target/debug/datasink insert users '{"id": 2, "name": "Bob", "email": "bob@example.com"}' -s http://127.0.0.1:50053

# Insert into analytics database
./target/debug/datasink insert events '{"id": 1, "event_type": "page_view", "timestamp": 1234567890}' -D analytics -s http://127.0.0.1:50053
./target/debug/datasink insert events '{"id": 2, "event_type": "click", "timestamp": 1234567891}' -D analytics -s http://127.0.0.1:50053

# Insert into logs database
./target/debug/datasink insert access_logs '{"id": 1, "ip": "192.168.1.1", "path": "/home", "timestamp": 1234567890}' -D logs -s http://127.0.0.1:50053
./target/debug/datasink insert access_logs '{"id": 2, "ip": "192.168.1.2", "path": "/api/data", "timestamp": 1234567891}' -D logs -s http://127.0.0.1:50053

# Query data from different databases
echo -e "\n🔍 Querying data from different databases..."

echo -e "\nUsers from default database:"
./target/debug/datasink query "SELECT * FROM users" -f table -s http://127.0.0.1:50053

echo -e "\nEvents from analytics database:"
./target/debug/datasink query "SELECT * FROM events" -f table -D analytics -s http://127.0.0.1:50053

echo -e "\nAccess logs from logs database:"
./target/debug/datasink query "SELECT * FROM access_logs" -f table -D logs -s http://127.0.0.1:50053

# Show statistics
echo -e "\n📊 Database statistics:"
./target/debug/datasink schema stats -s http://127.0.0.1:50053
./target/debug/datasink schema stats -D analytics -s http://127.0.0.1:50053
./target/debug/datasink schema stats -D logs -s http://127.0.0.1:50053

# Clean up
echo -e "\n🧹 Cleaning up..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

echo -e "\n✅ Multi-database test completed!"