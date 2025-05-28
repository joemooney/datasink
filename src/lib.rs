pub mod db;
pub mod schema;

pub mod proto {
    pub mod common {
        tonic::include_proto!("datasink.common");
    }
    pub mod admin {
        tonic::include_proto!("datasink.admin");
    }
    pub mod crud {
        tonic::include_proto!("datasink.crud");
    }
    tonic::include_proto!("datasink");
}

// Re-export commonly used types
pub use db::{Database, SqliteDatabase};