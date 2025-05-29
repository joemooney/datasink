use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use crate::db::{Database, DatabaseError, DatabaseManager};
use crate::grpc::conversions::*;
use crate::proto::data_sink_server::DataSink;
use crate::proto::admin::{
    CreateTableRequest, CreateTableResponse, DropTableRequest, DropTableResponse,
    ServerStatusRequest, ServerStatusResponse, DatabaseStatus,
    AddDatabaseRequest, AddDatabaseResponse,
};
use crate::proto::crud::{
    BatchInsertRequest, BatchInsertResponse, DeleteRequest, DeleteResponse,
    InsertRequest, InsertResponse, QueryRequest, QueryResponse, ResultSet,
    UpdateRequest, UpdateResponse, query_response,
};
use crate::proto::common::{Column as ProtoColumn, Error, Row};

pub struct DataSinkService {
    db_manager: Arc<DatabaseManager>,
    start_time: Instant,
}

impl DataSinkService {
    pub fn new_with_manager(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            start_time: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub async fn new(_db: Box<dyn Database>) -> Self {
        let manager = Arc::new(DatabaseManager::new());
        // Add the database as "default"
        if let Err(e) = manager.add_database("default".to_string(), "sqlite://default.db".to_string()).await {
            eprintln!("Warning: Failed to add default database to manager: {}", e);
        }
        Self::new_with_manager(manager)
    }

    async fn get_database(&self, database_name: Option<&str>) -> Result<Arc<RwLock<Box<dyn Database>>>, Status> {
        self.db_manager
            .get_database_or_default(database_name)
            .await
            .ok_or_else(|| {
                match database_name {
                    Some(name) => Status::not_found(format!("Database '{}' not found", name)),
                    None => Status::unavailable("No database connections available"),
                }
            })
    }

    fn db_error_to_status(err: DatabaseError) -> Status {
        match err {
            DatabaseError::TableAlreadyExists(table) => {
                Status::already_exists(format!("Table '{}' already exists", table))
            }
            DatabaseError::TableNotFound(table) => {
                Status::not_found(format!("Table '{}' not found", table))
            }
            DatabaseError::QueryError(msg) => Status::invalid_argument(msg),
            DatabaseError::ConnectionError(msg) => Status::unavailable(msg),
            _ => Status::internal(err.to_string()),
        }
    }
}

#[tonic::async_trait]
impl DataSink for DataSinkService {
    async fn create_table(
        &self,
        request: Request<CreateTableRequest>,
    ) -> Result<Response<CreateTableResponse>, Status> {
        let req = request.into_inner();

        let columns: Vec<_> = req.columns.into_iter().map(proto_to_column_def).collect();

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.create_table(&req.table_name, columns).await {
            Ok(_) => Ok(Response::new(CreateTableResponse {
                success: true,
                message: format!("Table '{}' created successfully", req.table_name),
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn drop_table(
        &self,
        request: Request<DropTableRequest>,
    ) -> Result<Response<DropTableResponse>, Status> {
        let req = request.into_inner();

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.drop_table(&req.table_name).await {
            Ok(_) => Ok(Response::new(DropTableResponse {
                success: true,
                message: format!("Table '{}' dropped successfully", req.table_name),
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn insert(
        &self,
        request: Request<InsertRequest>,
    ) -> Result<Response<InsertResponse>, Status> {
        let req = request.into_inner();

        let values = proto_values_to_db_values(req.values);

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.insert(&req.table_name, values).await {
            Ok(id) => Ok(Response::new(InsertResponse {
                success: true,
                message: "Insert successful".to_string(),
                inserted_id: id,
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        let req = request.into_inner();

        let values = proto_values_to_db_values(req.values);

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.update(&req.table_name, values, &req.where_clause).await {
            Ok(affected) => Ok(Response::new(UpdateResponse {
                success: true,
                message: format!("{} rows updated", affected),
                affected_rows: affected as i64,
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.delete(&req.table_name, &req.where_clause).await {
            Ok(affected) => Ok(Response::new(DeleteResponse {
                success: true,
                message: format!("{} rows deleted", affected),
                affected_rows: affected as i64,
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    type QueryStream = Pin<Box<dyn Stream<Item = Result<QueryResponse, Status>> + Send>>;

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::QueryStream>, Status> {
        let req = request.into_inner();

        let params = proto_values_to_db_values(req.parameters);

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.query_stream(&req.sql, params).await {
            Ok((columns, mut stream)) => {
                let proto_columns: Vec<ProtoColumn> = columns
                    .into_iter()
                    .map(|(name, col_type)| ProtoColumn {
                        name,
                        r#type: column_type_to_proto(&col_type) as i32,
                    })
                    .collect();

                let response_stream = Box::pin(async_stream::stream! {
                    // Send column information in the first response
                    yield Ok(QueryResponse {
                        response: Some(query_response::Response::ResultSet(ResultSet {
                            columns: proto_columns.clone(),
                            rows: vec![],
                        })),
                    });

                    // Stream rows
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(values) => {
                                let proto_values = db_values_to_proto_values(values);
                                yield Ok(QueryResponse {
                                    response: Some(query_response::Response::ResultSet(ResultSet {
                                        columns: vec![],
                                        rows: vec![Row { values: proto_values }],
                                    })),
                                });
                            }
                            Err(e) => {
                                yield Ok(QueryResponse {
                                    response: Some(query_response::Response::Error(Error {
                                        code: "QUERY_ERROR".to_string(),
                                        message: e.to_string(),
                                    })),
                                });
                                break;
                            }
                        }
                    }
                });

                Ok(Response::new(response_stream))
            }
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn batch_insert(
        &self,
        request: Request<BatchInsertRequest>,
    ) -> Result<Response<BatchInsertResponse>, Status> {
        let req = request.into_inner();

        let rows: Vec<_> = req
            .rows
            .into_iter()
            .map(|row| proto_values_to_db_values(row.values))
            .collect();

        let db_arc = self.get_database(if req.database.is_empty() { None } else { Some(&req.database) }).await?;
        let db = db_arc.read().await;
        match db.batch_insert(&req.table_name, rows).await {
            Ok(count) => Ok(Response::new(BatchInsertResponse {
                success: true,
                message: format!("{} rows inserted", count),
                inserted_count: count as i64,
            })),
            Err(e) => Err(Self::db_error_to_status(e)),
        }
    }

    async fn get_server_status(
        &self,
        _request: Request<ServerStatusRequest>,
    ) -> Result<Response<ServerStatusResponse>, Status> {
        let databases = self.db_manager.list_databases().await;
        
        let db_statuses: Vec<DatabaseStatus> = databases
            .into_iter()
            .map(|db_info| DatabaseStatus {
                name: db_info.name,
                url: db_info.url,
                connected: db_info.connected,
                connection_time: db_info.connection_time
                    .map(|t| t.timestamp())
                    .unwrap_or(0),
                active_connections: 1, // For now, assume 1 connection per database
            })
            .collect();

        Ok(Response::new(ServerStatusResponse {
            server_running: true,
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
            databases: db_statuses,
        }))
    }

    async fn add_database(
        &self,
        request: Request<AddDatabaseRequest>,
    ) -> Result<Response<AddDatabaseResponse>, Status> {
        let req = request.into_inner();
        
        // Validate database name
        if req.name.is_empty() {
            return Ok(Response::new(AddDatabaseResponse {
                success: false,
                message: "Database name cannot be empty".to_string(),
            }));
        }
        
        // Add create mode if not already present in the URL for SQLite
        let db_url = if req.url.starts_with("sqlite://") && !req.url.contains("?") {
            format!("{}?mode=rwc", req.url)
        } else {
            req.url.clone()
        };
        
        match self.db_manager.add_database(req.name.clone(), db_url).await {
            Ok(_) => Ok(Response::new(AddDatabaseResponse {
                success: true,
                message: format!("Database '{}' added successfully", req.name),
            })),
            Err(e) => Ok(Response::new(AddDatabaseResponse {
                success: false,
                message: format!("Failed to add database '{}': {}", req.name, e),
            })),
        }
    }
}
