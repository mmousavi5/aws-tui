use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

use super::cloudwatch_client::{CloudWatchClient, CloudWatchClientError};
use super::dynamo_client::{DynamoDBClient, DynamoDBClientError};
use super::s3_client::{S3Client, S3ClientError};

/// Error types specific to TabClients operations
///
/// Wraps various AWS service client errors into a single error type
#[derive(Error, Debug)]
pub enum TabClientsError {
    /// Errors from S3 client operations
    #[error("S3 client error: {0}")]
    S3Error(#[from] S3ClientError),

    /// Errors from DynamoDB client operations
    #[error("DynamoDB client error: {0}")]
    DynamoDBError(#[from] DynamoDBClientError),

    /// Errors from CloudWatch client operations
    #[error("CloudWatch client error: {0}")]
    CloudWatchError(#[from] CloudWatchClientError),

    /// Direct AWS SDK errors for S3
    #[error("AWS S3 SDK error: {0}")]
    AWSS3Error(#[from] aws_sdk_s3::Error),

    /// Direct AWS SDK errors for DynamoDB
    #[error("AWS DynamoDB SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),

    /// Direct AWS SDK errors for CloudWatch
    #[error("AWS CloudWatch SDK error: {0}")]
    AWSCloudWatchError(#[from] aws_sdk_cloudwatch::Error),
}

/// Manages AWS service clients for a specific tab
///
/// Provides lazy initialization and caching of service clients
/// using the specified AWS profile and region
pub struct TabClients {
    /// Cached S3 client instance
    s3_client: Option<Arc<Mutex<S3Client>>>,

    /// Cached DynamoDB client instance
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,

    /// Cached CloudWatch client instance
    cloudwatch_client: Option<Arc<Mutex<CloudWatchClient>>>,

    /// AWS profile name used for authentication
    profile: String,

    /// AWS region for all service clients
    region: String,
}

impl TabClients {
    /// Creates a new TabClients instance with the specified profile and region
    pub fn new(profile: String, region: String) -> Self {
        Self {
            s3_client: None,
            dynamodb_client: None,
            cloudwatch_client: None,
            profile,
            region,
        }
    }

    /// Updates the profile and invalidates all existing clients
    ///
    /// This forces new clients to be created on next request with the new profile
    pub fn set_profile(&mut self, profile: String) {
        if self.profile != profile {
            self.profile = profile;
            self.s3_client = None;
            self.dynamodb_client = None;
            self.cloudwatch_client = None;
        }
    }

    /// Gets or initializes an S3 client
    ///
    /// Creates a new client if none exists, otherwise returns the cached instance
    pub async fn get_s3_client(&mut self) -> Result<Arc<Mutex<S3Client>>, TabClientsError> {
        if self.s3_client.is_none() {
            let client = S3Client::new(self.profile.clone(), self.region.clone()).await?;
            self.s3_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.s3_client.as_ref().unwrap().clone())
    }

    /// Gets or initializes a DynamoDB client
    ///
    /// Creates a new client if none exists, otherwise returns the cached instance
    pub async fn get_dynamodb_client(
        &mut self,
    ) -> Result<Arc<Mutex<DynamoDBClient>>, TabClientsError> {
        if self.dynamodb_client.is_none() {
            let client = DynamoDBClient::new(self.profile.clone(), self.region.clone()).await?;
            self.dynamodb_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.dynamodb_client.as_ref().unwrap().clone())
    }

    /// Gets or initializes a CloudWatch client
    ///
    /// Creates a new client if none exists, otherwise returns the cached instance
    pub async fn get_cloudwatch_client(
        &mut self,
    ) -> Result<Arc<Mutex<CloudWatchClient>>, TabClientsError> {
        if self.cloudwatch_client.is_none() {
            let client = CloudWatchClient::new(self.profile.clone(), self.region.clone()).await?;
            self.cloudwatch_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.cloudwatch_client.as_ref().unwrap().clone())
    }
}
