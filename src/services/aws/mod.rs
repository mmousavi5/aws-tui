//! AWS Services Module
//!
//! Contains client implementations for various AWS services used in the TUI application.
//! Provides unified error handling and client management.

// Client implementations for specific AWS services
pub mod cloudwatch_client;
pub mod dynamo_client;
pub mod s3_client;
mod tab_clients;

// Re-export TabClients for profile and region management
pub use tab_clients::TabClients;

// Import individual service error types for unified error handling
use super::aws::cloudwatch_client::CloudWatchClientError;
use super::aws::dynamo_client::DynamoDBClientError;
use super::aws::s3_client::S3ClientError;
use thiserror::Error;

/// Unified error type for all AWS service operations
///
/// Wraps service-specific errors into a single type for simpler error handling
/// in the application layer.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Errors from DynamoDB operations
    #[error("AWS DynamoDBSDK error: {0}")]
    AWSDynamoDBError(#[from] DynamoDBClientError),

    /// Errors from S3 operations
    #[error("AWS S3SDK error: {0}")]
    AWSS3Error(#[from] S3ClientError),

    /// Errors from CloudWatch operations
    #[error("AWS CloudWatch error: {0}")]
    AWSCloudWatchError(#[from] CloudWatchClientError),
}
