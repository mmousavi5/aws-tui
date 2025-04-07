pub mod cloudwatch_client;
pub mod dynamo_client;
pub mod s3_client;
mod tab_clients;

pub use tab_clients::TabClients;

use super::aws::cloudwatch_client::CloudWatchClientError;
use super::aws::dynamo_client::DynamoDBClientError;
use super::aws::s3_client::S3ClientError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("AWS DynamoDBSDK error: {0}")]
    AWSDynamoDBError(#[from] DynamoDBClientError),
    #[error("AWS S3SDK error: {0}")]
    AWSS3Error(#[from] S3ClientError),
    #[error("AWS CloudWatch error: {0}")]
    AWSCloudWatchError(#[from] CloudWatchClientError),
}
