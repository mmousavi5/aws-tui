pub mod dynamo_client;
pub mod s3_client;
pub mod cloudwatch_client;
mod tab_clients;

pub use tab_clients::{TabClients, TabClientsError};

use super::aws::dynamo_client::DynamoDBClientError;
use super::aws::s3_client::S3ClientError;
use super::aws::cloudwatch_client::CloudWatchClientError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("AWS DynamoDBSDK error: {0}")]
    AWSDynamoDBError(#[from] DynamoDBClientError),
    #[error("AWS S3SDK error: {0}")]
    AWSS3Error(#[from] S3ClientError),
    #[error("AWS CloudWatch error: {0}")]
    AWSCloudWatchError(#[from] CloudWatchClientError),
    #[error("Secret not found")]
    GeneralError,
}