pub mod dynamo_client;
pub mod s3_client;
use thiserror::Error;
use crate::services::aws::dynamo_client::AWSDynamoDBError;
use crate::services::aws::s3_client::AWSS3Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("AWS DynamoDBSDK error: {0}")]
    AWSDynamoDBError(#[from] AWSDynamoDBError),
    #[error("AWS S3SDK error: {0}")]
    AWSS3Error(#[from] AWSS3Error),
    #[error("Secret not found")]
    GeneralError,
}
