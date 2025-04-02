use aws_config::{defaults, BehaviorVersion, Region};
use aws_sdk_dynamodb::{Client, Error as DynamoDBError};
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::list_tables::ListTablesError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DynamoDBClientError {
    #[error("AWS SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),
    #[error("ListTables error: {0}")]
    ListTablesError(#[from] SdkError<ListTablesError, HttpResponse>),
}

pub struct DynamoDBClient {
    client: Client,
}

impl DynamoDBClient {
    pub async fn new(profile: String, region: String) -> Result<Self, DynamoDBError> {
        let config = defaults(BehaviorVersion::latest())
            .profile_name(profile)
            .region(Region::new(region))
            .load()
            .await;
        Ok(Self { 
            client: Client::new(&config) 
        })
    }

    pub async fn list_tables(&self) -> Result<Vec<String>, DynamoDBClientError> {
        let output = self.client.list_tables().send().await?;
        Ok(output.table_names().to_vec())
    }
}