#!/usr/bin/env node

// Simple HTTP proxy server for DataSink gRPC service
// This allows web clients to communicate with the gRPC server

const http = require('http');
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');
const path = require('path');

// Configuration
const HTTP_PORT = 8080;
const GRPC_SERVER = 'localhost:50051';

// Load proto file
const PROTO_PATH = path.join(__dirname, 'proto', 'datasink.proto');
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true
});

const datasink = grpc.loadPackageDefinition(packageDefinition).datasink;
const client = new datasink.DataSink(GRPC_SERVER, grpc.credentials.createInsecure());

// CORS headers
const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type',
  'Content-Type': 'application/json'
};

// HTTP server
const server = http.createServer(async (req, res) => {
  // Handle CORS preflight
  if (req.method === 'OPTIONS') {
    res.writeHead(200, corsHeaders);
    res.end();
    return;
  }

  // Parse URL
  const url = new URL(req.url, `http://${req.headers.host}`);
  const endpoint = url.pathname;

  // Only accept POST requests
  if (req.method !== 'POST') {
    res.writeHead(405, corsHeaders);
    res.end(JSON.stringify({ error: 'Method not allowed' }));
    return;
  }

  // Collect request body
  let body = '';
  req.on('data', chunk => { body += chunk; });
  req.on('end', async () => {
    try {
      const data = JSON.parse(body);

      switch (endpoint) {
        case '/query':
          handleQuery(data, res);
          break;
        case '/insert':
          handleInsert(data, res);
          break;
        case '/update':
          handleUpdate(data, res);
          break;
        case '/delete':
          handleDelete(data, res);
          break;
        default:
          res.writeHead(404, corsHeaders);
          res.end(JSON.stringify({ error: 'Endpoint not found' }));
      }
    } catch (error) {
      res.writeHead(400, corsHeaders);
      res.end(JSON.stringify({ error: error.message }));
    }
  });
});

// Handler functions
function handleQuery(data, res) {
  const request = {
    sql: data.sql,
    parameters: data.parameters || {},
    database: data.database
  };

  const stream = client.Query(request);
  const results = [];

  stream.on('data', (row) => {
    const rowData = {};
    row.values.forEach((value, index) => {
      const colName = row.columns[index] || `col${index}`;
      rowData[colName] = parseValue(value);
    });
    results.push(rowData);
  });

  stream.on('end', () => {
    res.writeHead(200, corsHeaders);
    res.end(JSON.stringify({ success: true, data: results }));
  });

  stream.on('error', (error) => {
    res.writeHead(500, corsHeaders);
    res.end(JSON.stringify({ success: false, error: error.message }));
  });
}

function handleInsert(data, res) {
  const request = {
    table_name: data.table_name,
    values: data.values,
    database: data.database
  };

  client.Insert(request, (error, response) => {
    if (error) {
      res.writeHead(500, corsHeaders);
      res.end(JSON.stringify({ success: false, error: error.message }));
    } else {
      res.writeHead(200, corsHeaders);
      res.end(JSON.stringify({ 
        success: true, 
        inserted_id: response.inserted_id,
        affected_rows: response.affected_rows 
      }));
    }
  });
}

function handleUpdate(data, res) {
  const request = {
    table_name: data.table_name,
    values: data.values,
    where_clause: data.where_clause,
    database: data.database
  };

  client.Update(request, (error, response) => {
    if (error) {
      res.writeHead(500, corsHeaders);
      res.end(JSON.stringify({ success: false, error: error.message }));
    } else {
      res.writeHead(200, corsHeaders);
      res.end(JSON.stringify({ 
        success: true, 
        affected_rows: response.affected_rows 
      }));
    }
  });
}

function handleDelete(data, res) {
  const request = {
    table_name: data.table_name,
    where_clause: data.where_clause,
    database: data.database
  };

  client.Delete(request, (error, response) => {
    if (error) {
      res.writeHead(500, corsHeaders);
      res.end(JSON.stringify({ success: false, error: error.message }));
    } else {
      res.writeHead(200, corsHeaders);
      res.end(JSON.stringify({ 
        success: true, 
        affected_rows: response.affected_rows 
      }));
    }
  });
}

// Helper to parse values
function parseValue(value) {
  if (value.int_value !== undefined) return value.int_value;
  if (value.float_value !== undefined) return value.float_value;
  if (value.string_value !== undefined) return value.string_value;
  if (value.bool_value !== undefined) return value.bool_value;
  if (value.null_value !== undefined) return null;
  return value.string_value || null;
}

// Start server
server.listen(HTTP_PORT, () => {
  console.log(`HTTP proxy server running on http://localhost:${HTTP_PORT}`);
  console.log(`Proxying to gRPC server at ${GRPC_SERVER}`);
  console.log('\nEndpoints:');
  console.log('  POST /query');
  console.log('  POST /insert');
  console.log('  POST /update');
  console.log('  POST /delete');
});