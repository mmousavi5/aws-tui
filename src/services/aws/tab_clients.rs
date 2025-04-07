use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

use super::dynamo_client::{DynamoDBClient, DynamoDBClientError};
use super::s3_client::{S3Client, S3ClientError};
use super::cloudwatch_client::{CloudWatchClient, CloudWatchClientError};

#[derive(Error, Debug)]
pub enum TabClientsError {
    #[error("S3 client error: {0}")]
    S3Error(#[from] S3ClientError),
    #[error("DynamoDB client error: {0}")]
    DynamoDBError(#[from] DynamoDBClientError),
    #[error("CloudWatch client error: {0}")]
    CloudWatchError(#[from] CloudWatchClientError),
    #[error("AWS S3 SDK error: {0}")]
    AWSS3Error(#[from] aws_sdk_s3::Error),
    #[error("AWS DynamoDB SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),
    #[error("AWS CloudWatch SDK error: {0}")]
    AWSCloudWatchError(#[from] aws_sdk_cloudwatch::Error),
    #[error("Client initialization error: {0}")]
    InitError(String),
}

pub struct TabClients {
    s3_client: Option<Arc<Mutex<S3Client>>>,
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,
    cloudwatch_client: Option<Arc<Mutex<CloudWatchClient>>>,
    profile: String,
    region: String,
}

impl TabClients {
    pub fn new(profile: String, region: String) -> Self {
        Self {
            s3_client: None,
            dynamodb_client: None,
            cloudwatch_client: None,
            profile,
            region,
        }
    }

    pub fn set_profile(&mut self, profile: String) {
        if self.profile != profile {
            self.profile = profile;
            self.s3_client = None;
            self.dynamodb_client = None;
            self.cloudwatch_client = None;
        }
    }

    pub fn set_region(&mut self, region: String) {
        if self.region != region {
            self.region = region;
            self.s3_client = None;
            self.dynamodb_client = None;
            self.cloudwatch_client = None;
        }
    }

    pub async fn get_s3_client(&mut self) -> Result<Arc<Mutex<S3Client>>, TabClientsError> {
        if self.s3_client.is_none() {
            let client = S3Client::new(self.profile.clone(), self.region.clone()).await?;
            self.s3_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.s3_client.as_ref().unwrap().clone())
    }

    pub async fn get_dynamodb_client(
        &mut self,
    ) -> Result<Arc<Mutex<DynamoDBClient>>, TabClientsError> {
        if self.dynamodb_client.is_none() {
            let client = DynamoDBClient::new(self.profile.clone(), self.region.clone()).await?;
            self.dynamodb_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.dynamodb_client.as_ref().unwrap().clone())
    }
    
    pub async fn get_cloudwatch_client(
        &mut self,
    ) -> Result<Arc<Mutex<CloudWatchClient>>, TabClientsError> {
        if self.cloudwatch_client.is_none() {
            let client = CloudWatchClient::new(self.profile.clone(), self.region.clone()).await?;
            self.cloudwatch_client = Some(Arc::new(Mutex::new(client)));
        }
        Ok(self.cloudwatch_client.as_ref().unwrap().clone())
    }

    pub async fn list_s3_buckets(&mut self) -> Result<Vec<String>, TabClientsError> {
        let client = self.get_s3_client().await?;
        let client = client.lock().await;
        Ok(client.list_buckets().await?)
    }

    pub async fn list_dynamodb_tables(&mut self) -> Result<Vec<String>, TabClientsError> {
        let client = self.get_dynamodb_client().await?;
        let client = client.lock().await;
        Ok(client.list_tables().await?)
    }

}