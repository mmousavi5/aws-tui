pub mod dynamo_client;

use async_trait::async_trait;
use thiserror::Error;
use crate::services::aws::dynamo_client::AWSDynamoDBError;
use crate::services::aws::dynamo_client::DynamoDBClient;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("AWS DynamoDBSDK error: {0}")]
    AWSDynamoDBError(#[from] AWSDynamoDBError),
    #[error("Secret not found")]
    GeneralError,
}
