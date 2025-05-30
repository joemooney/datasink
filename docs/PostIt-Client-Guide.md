# PostIt Web Client Implementation Guide

This guide provides comprehensive information for building a web-based frontend client for the PostIt notes management system using JavaScript/TypeScript.

## Table of Contents
- [Overview](#overview)
- [Database Schema](#database-schema)
- [API Communication](#api-communication)
- [Authentication & Users](#authentication--users)
- [Core Features](#core-features)
- [Data Models](#data-models)
- [Example API Calls](#example-api-calls)
- [UI/UX Recommendations](#uiux-recommendations)
- [Development Setup](#development-setup)

## Overview

PostIt is a note management system that allows users to:
- Create, read, update, and delete notes
- Organize notes with tags
- Set priorities and statuses
- Track who created each note
- Filter and search notes

The backend uses DataSink, a gRPC-based database service, but web clients will typically communicate through a REST API proxy or gRPC-Web.

## Database Schema

### Tables

#### 1. notes
The main table storing all post-it notes.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | INTEGER | PRIMARY KEY, AUTO_INCREMENT | Unique identifier |
| title | TEXT | NOT NULL | Note title |
| description | TEXT | NULLABLE | Detailed note content |
| created_at | TIMESTAMP | NOT NULL, DEFAULT CURRENT_TIMESTAMP | Creation timestamp |
| created_by | TEXT | NULLABLE | Username of creator |
| status | TEXT | NOT NULL, DEFAULT 'open' | Note status |
| priority | TEXT | NOT NULL, DEFAULT 'medium' | Note priority |
| url | TEXT | NULLABLE | Associated URL |

**Valid Status Values:**
- `open` - Active note requiring attention
- `working` - Currently being worked on
- `closed` - Completed
- `archived` - No longer active but kept for reference

**Valid Priority Values:**
- `low` - Can be addressed later
- `medium` - Normal priority
- `high` - Should be addressed soon
- `critical` - Requires immediate attention

#### 2. tags
Available tags for categorizing notes.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| id | INTEGER | PRIMARY KEY, AUTO_INCREMENT | Unique identifier |
| name | TEXT | NOT NULL, UNIQUE | Tag name |
| color | TEXT | NULLABLE | Hex color code (e.g., "#FF6B6B") |

**Predefined Tags:**
- todo (#FF6B6B)
- idea (#4ECDC4)
- bug (#FF4757)
- feature (#45B7D1)
- documentation (#96CEB4)
- research (#DDA0DD)
- meeting (#FFB6C1)
- reminder (#FFA07A)

#### 3. note_tags
Junction table for many-to-many relationship between notes and tags.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| note_id | INTEGER | NOT NULL, FOREIGN KEY | References notes.id |
| tag_id | INTEGER | NOT NULL, FOREIGN KEY | References tags.id |

## API Communication

### Important: Web Browsers Cannot Connect Directly to gRPC

DataSink runs a native gRPC server (default port 50051), but web browsers cannot connect directly to gRPC servers. You need a proxy or gateway:

### Connection Options

1. **gRPC-Web Proxy** (For gRPC-style communication)
   
   You need to run a gRPC-Web proxy like Envoy that translates between gRPC-Web and gRPC:
   
   ```yaml
   # envoy.yaml example
   static_resources:
     listeners:
     - name: listener_0
       address:
         socket_address: { address: 0.0.0.0, port_value: 8080 }
       filter_chains:
       - filters:
         - name: envoy.filters.network.http_connection_manager
           typed_config:
             "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
             codec_type: auto
             stat_prefix: ingress_http
             route_config:
               name: local_route
               virtual_hosts:
               - name: local_service
                 domains: ["*"]
                 routes:
                 - match: { prefix: "/" }
                   route:
                     cluster: datasink_service
                     timeout: 0s
                     max_stream_duration:
                       grpc_timeout_header_max: 0s
                 cors:
                   allow_origin_string_match:
                   - prefix: "*"
                   allow_methods: GET, PUT, DELETE, POST, OPTIONS
                   allow_headers: keep-alive,user-agent,cache-control,content-type,content-transfer-encoding,custom-header-1,x-accept-content-transfer-encoding,x-accept-response-streaming,x-user-agent,x-grpc-web,grpc-timeout
                   max_age: "1728000"
                   expose_headers: custom-header-1,grpc-status,grpc-message
             http_filters:
             - name: envoy.filters.http.grpc_web
               typed_config:
                 "@type": type.googleapis.com/envoy.extensions.filters.http.grpc_web.v3.GrpcWeb
             - name: envoy.filters.http.cors
               typed_config:
                 "@type": type.googleapis.com/envoy.extensions.filters.http.cors.v3.Cors
             - name: envoy.filters.http.router
               typed_config:
                 "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router
     clusters:
     - name: datasink_service
       connect_timeout: 0.25s
       type: logical_dns
       http2_protocol_options: {}
       lb_policy: round_robin
       load_assignment:
         cluster_name: datasink_service
         endpoints:
         - lb_endpoints:
           - endpoint:
               address:
                 socket_address:
                   address: 127.0.0.1
                   port_value: 50051  # Your DataSink gRPC server port
   ```
   
   Then in your JavaScript:
   ```javascript
   import { DataSinkClient } from './generated/datasink_grpc_web_pb';
   const client = new DataSinkClient('http://localhost:8080'); // Envoy proxy port
   ```

2. **REST API Gateway** (Easier - Recommended for quick start)
   
   Create a simple Node.js/Express gateway that translates REST to gRPC:
   
   ```javascript
   // rest-gateway.js
   const express = require('express');
   const cors = require('cors');
   const grpc = require('@grpc/grpc-js');
   const protoLoader = require('@grpc/proto-loader');
   
   const app = express();
   app.use(cors());
   app.use(express.json());
   
   // Load DataSink proto
   const packageDefinition = protoLoader.loadSync(
     'path/to/datasink.proto',
     { keepCase: true, longs: String, enums: String, defaults: true, oneofs: true }
   );
   const datasink = grpc.loadPackageDefinition(packageDefinition).datasink;
   
   // Connect to DataSink gRPC server
   const client = new datasink.DataSink(
     '127.0.0.1:50051', 
     grpc.credentials.createInsecure()
   );
   
   // REST endpoint for queries
   app.post('/api/query', (req, res) => {
     const { sql, parameters = {}, database = '' } = req.body;
     
     const request = { sql, parameters, database };
     const call = client.query(request);
     
     let results = { columns: [], rows: [] };
     
     call.on('data', (response) => {
       if (response.result_set) {
         if (response.result_set.columns.length > 0) {
           results.columns = response.result_set.columns;
         }
         results.rows.push(...response.result_set.rows);
       }
     });
     
     call.on('end', () => {
       res.json(results);
     });
     
     call.on('error', (error) => {
       res.status(500).json({ error: error.message });
     });
   });
   
   // Similar endpoints for insert, update, delete...
   
   app.listen(3000, () => {
     console.log('REST gateway listening on port 3000');
   });
   ```
   
   Then in your frontend:
   ```javascript
   const API_BASE = 'http://localhost:3000/api';
   
   async function query(sql, params = {}) {
     const response = await fetch(`${API_BASE}/query`, {
       method: 'POST',
       headers: { 'Content-Type': 'application/json' },
       body: JSON.stringify({ sql, parameters: params })
     });
     return response.json();
   }
   ```

3. **Direct CLI Usage** (For development/testing only)
   
   During development, you can use the DataSink CLI and pipe JSON output:
   ```bash
   # Start server
   datasink server start -b 127.0.0.1:50051
   
   # Query via CLI with JSON output
   datasink query "SELECT * FROM notes" -f json
   ```

### Database Selection

If using multiple databases, include the database parameter:
```javascript
{
  sql: "SELECT * FROM notes",
  database: "postit"  // Optional, defaults to "default"
}
```

## Authentication & Users

The `created_by` field in notes tracks who created each note. Your client should:

1. **Get Current User**
   ```javascript
   const currentUser = getCurrentUser(); // From your auth system
   ```

2. **Include in Creates**
   ```javascript
   const newNote = {
     title: "New Task",
     description: "Task details",
     priority: "medium",
     status: "open",
     created_by: currentUser.username
   };
   ```

3. **Filter by User**
   ```javascript
   // My notes
   const myNotes = await query(
     "SELECT * FROM notes WHERE created_by = ?",
     { created_by: currentUser.username }
   );
   ```

## Core Features

### 1. Note Management

#### Create Note
```javascript
async function createNote(note) {
  const sql = `
    INSERT INTO notes (title, description, priority, status, created_by, url)
    VALUES (?, ?, ?, ?, ?, ?)
  `;
  
  return await insert('notes', {
    title: note.title,
    description: note.description || null,
    priority: note.priority || 'medium',
    status: note.status || 'open',
    created_by: note.created_by,
    url: note.url || null
  });
}
```

#### Update Note
```javascript
async function updateNote(id, updates) {
  return await update('notes', updates, `id = ${id}`);
}
```

#### Delete Note
```javascript
async function deleteNote(id) {
  // First remove tag associations
  await deleteWhere('note_tags', `note_id = ${id}`);
  // Then delete the note
  await deleteWhere('notes', `id = ${id}`);
}
```

### 2. Tag Management

#### Get Note Tags
```javascript
async function getNoteTags(noteId) {
  const sql = `
    SELECT t.* FROM tags t
    JOIN note_tags nt ON t.id = nt.tag_id
    WHERE nt.note_id = ?
  `;
  return await query(sql, { note_id: noteId });
}
```

#### Add Tag to Note
```javascript
async function addTagToNote(noteId, tagId) {
  return await insert('note_tags', {
    note_id: noteId,
    tag_id: tagId
  });
}
```

#### Remove Tag from Note
```javascript
async function removeTagFromNote(noteId, tagId) {
  return await deleteWhere('note_tags', 
    `note_id = ${noteId} AND tag_id = ${tagId}`
  );
}
```

### 3. Queries & Filters

#### Get Notes with Tags
```javascript
async function getNotesWithTags() {
  const sql = `
    SELECT 
      n.*,
      GROUP_CONCAT(t.name) as tag_names,
      GROUP_CONCAT(t.color) as tag_colors
    FROM notes n
    LEFT JOIN note_tags nt ON n.id = nt.note_id
    LEFT JOIN tags t ON nt.tag_id = t.id
    GROUP BY n.id
    ORDER BY n.created_at DESC
  `;
  return await query(sql);
}
```

#### Filter by Status
```javascript
async function getNotesByStatus(status) {
  return await query(
    "SELECT * FROM notes WHERE status = ? ORDER BY priority DESC, created_at DESC",
    { status }
  );
}
```

#### Filter by Priority
```javascript
async function getHighPriorityNotes() {
  return await query(
    "SELECT * FROM notes WHERE priority IN ('high', 'critical') AND status = 'open'"
  );
}
```

#### Search Notes
```javascript
async function searchNotes(searchTerm) {
  const sql = `
    SELECT * FROM notes 
    WHERE title LIKE ? OR description LIKE ?
    ORDER BY created_at DESC
  `;
  const pattern = `%${searchTerm}%`;
  return await query(sql, { 
    title_pattern: pattern, 
    desc_pattern: pattern 
  });
}
```

## Data Models

### TypeScript Interfaces

```typescript
interface Note {
  id: number;
  title: string;
  description?: string;
  created_at: string; // ISO timestamp
  created_by?: string;
  status: 'open' | 'working' | 'closed' | 'archived';
  priority: 'low' | 'medium' | 'high' | 'critical';
  url?: string;
}

interface Tag {
  id: number;
  name: string;
  color?: string; // Hex color code
}

interface NoteTag {
  note_id: number;
  tag_id: number;
}

interface NoteWithTags extends Note {
  tags: Tag[];
}
```

### JavaScript Classes

```javascript
class Note {
  constructor(data) {
    this.id = data.id;
    this.title = data.title;
    this.description = data.description || '';
    this.created_at = new Date(data.created_at);
    this.created_by = data.created_by || 'anonymous';
    this.status = data.status || 'open';
    this.priority = data.priority || 'medium';
    this.url = data.url || null;
    this.tags = data.tags || [];
  }

  isPastDue() {
    // Implement your business logic
    return this.status === 'open' && this.priority === 'critical';
  }

  getStatusIcon() {
    const icons = {
      'open': '📋',
      'working': '⚡',
      'closed': '✅',
      'archived': '📦'
    };
    return icons[this.status] || '📄';
  }

  getPriorityClass() {
    return `priority-${this.priority}`;
  }
}
```

## Example API Calls

### Complete CRUD Example

```javascript
class PostItAPI {
  constructor(baseUrl) {
    this.baseUrl = baseUrl;
  }

  async query(sql, parameters = {}, database = null) {
    const body = { sql, parameters };
    if (database) body.database = database;
    
    const response = await fetch(`${this.baseUrl}/query`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    
    return response.json();
  }

  async insert(table, values, database = null) {
    const body = { table_name: table, values };
    if (database) body.database = database;
    
    const response = await fetch(`${this.baseUrl}/insert`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    
    return response.json();
  }

  async update(table, values, whereClause, database = null) {
    const body = { table_name: table, values, where_clause: whereClause };
    if (database) body.database = database;
    
    const response = await fetch(`${this.baseUrl}/update`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    
    return response.json();
  }

  async delete(table, whereClause, database = null) {
    const body = { table_name: table, where_clause: whereClause };
    if (database) body.database = database;
    
    const response = await fetch(`${this.baseUrl}/delete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    
    return response.json();
  }
}

// Usage
const api = new PostItAPI('http://localhost:3000/api');

// Get all open notes
const openNotes = await api.query(
  "SELECT * FROM notes WHERE status = 'open' ORDER BY priority DESC"
);

// Create a new note
const newNote = await api.insert('notes', {
  title: 'Review PR #123',
  description: 'Code review needed for authentication changes',
  priority: 'high',
  status: 'open',
  created_by: 'john.doe'
});

// Update note status
await api.update('notes', 
  { status: 'working' }, 
  `id = ${noteId}`
);

// Delete a note
await api.delete('notes', `id = ${noteId}`);
```

## UI/UX Recommendations

### 1. Note Cards
Display notes as cards with visual indicators:
```html
<div class="note-card priority-high status-open">
  <div class="note-header">
    <h3>Fix login timeout issue</h3>
    <span class="priority-badge">Critical</span>
  </div>
  <p class="note-description">Users are being logged out...</p>
  <div class="note-footer">
    <div class="tags">
      <span class="tag" style="background-color: #FF4757">bug</span>
      <span class="tag" style="background-color: #FF6B6B">todo</span>
    </div>
    <span class="created-by">by alice</span>
  </div>
</div>
```

### 2. Status Workflow
Implement drag-and-drop between status columns:
```javascript
const statusColumns = ['open', 'working', 'closed', 'archived'];

function moveNoteToStatus(noteId, newStatus) {
  return api.update('notes', 
    { status: newStatus }, 
    `id = ${noteId}`
  );
}
```

### 3. Priority Colors
CSS classes for visual priority:
```css
.priority-low { border-left: 4px solid #95a5a6; }
.priority-medium { border-left: 4px solid #3498db; }
.priority-high { border-left: 4px solid #f39c12; }
.priority-critical { border-left: 4px solid #e74c3c; }
```

### 4. Tag Management
Interactive tag selector:
```javascript
class TagSelector {
  constructor(tags, selectedTagIds = []) {
    this.tags = tags;
    this.selected = new Set(selectedTagIds);
  }

  toggle(tagId) {
    if (this.selected.has(tagId)) {
      this.selected.delete(tagId);
    } else {
      this.selected.add(tagId);
    }
    this.render();
  }

  render() {
    return this.tags.map(tag => `
      <button 
        class="tag-button ${this.selected.has(tag.id) ? 'selected' : ''}"
        style="background-color: ${tag.color}"
        onclick="tagSelector.toggle(${tag.id})"
      >
        ${tag.name}
      </button>
    `).join('');
  }
}
```

### 5. Real-time Search
Debounced search implementation:
```javascript
let searchTimeout;

function searchNotes(term) {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(async () => {
    const results = await api.query(
      `SELECT * FROM notes 
       WHERE title LIKE '%${term}%' OR description LIKE '%${term}%'
       ORDER BY created_at DESC`
    );
    displayResults(results);
  }, 300);
}
```

## Development Setup

### 1. Project Structure
```
postit-client/
├── src/
│   ├── api/
│   │   ├── client.js      # API client
│   │   └── models.js      # Data models
│   ├── components/
│   │   ├── NoteCard.js    # Note display component
│   │   ├── NoteForm.js    # Create/edit form
│   │   ├── TagSelector.js # Tag selection
│   │   └── StatusBoard.js # Kanban-style board
│   ├── utils/
│   │   ├── auth.js        # Authentication helpers
│   │   └── filters.js     # Query builders
│   └── app.js             # Main application
├── styles/
│   └── main.css           # Styles
└── index.html             # Entry point
```

### 2. Environment Configuration
```javascript
// config.js
const config = {
  API_URL: process.env.API_URL || 'http://localhost:8080',
  DATABASE: process.env.DATABASE || 'postit',
  DEFAULT_USER: process.env.DEFAULT_USER || 'guest'
};
```

### 3. Error Handling
```javascript
class APIError extends Error {
  constructor(message, status) {
    super(message);
    this.status = status;
  }
}

async function apiCall(url, options) {
  try {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new APIError(response.statusText, response.status);
    }
    return await response.json();
  } catch (error) {
    console.error('API Error:', error);
    showNotification('error', error.message);
    throw error;
  }
}
```

### 4. State Management
```javascript
class PostItStore {
  constructor() {
    this.notes = [];
    this.tags = [];
    this.filters = {
      status: null,
      priority: null,
      tags: [],
      search: ''
    };
  }

  async loadNotes() {
    const sql = this.buildFilterQuery();
    this.notes = await api.query(sql);
    this.notifyListeners();
  }

  buildFilterQuery() {
    let conditions = [];
    
    if (this.filters.status) {
      conditions.push(`status = '${this.filters.status}'`);
    }
    
    if (this.filters.priority) {
      conditions.push(`priority = '${this.filters.priority}'`);
    }
    
    if (this.filters.search) {
      conditions.push(`(title LIKE '%${this.filters.search}%' 
                       OR description LIKE '%${this.filters.search}%')`);
    }
    
    const whereClause = conditions.length > 0 
      ? `WHERE ${conditions.join(' AND ')}` 
      : '';
    
    return `
      SELECT n.*, GROUP_CONCAT(t.name) as tag_names
      FROM notes n
      LEFT JOIN note_tags nt ON n.id = nt.note_id
      LEFT JOIN tags t ON nt.tag_id = t.id
      ${whereClause}
      GROUP BY n.id
      ORDER BY 
        CASE priority 
          WHEN 'critical' THEN 1 
          WHEN 'high' THEN 2 
          WHEN 'medium' THEN 3 
          WHEN 'low' THEN 4 
        END,
        created_at DESC
    `;
  }
}
```

## Additional Resources

- [DataSink Documentation](https://github.com/joemooney/datasink)
- [gRPC-Web Guide](https://grpc.io/docs/platforms/web/quickstart/)
- [SQLite SQL Reference](https://www.sqlite.org/lang.html)

## Support

For questions about:
- Database schema: See `schemas/PostIt.schema`
- API endpoints: Check DataSink server documentation
- Frontend implementation: Create an issue in your project repository