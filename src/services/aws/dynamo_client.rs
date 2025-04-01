use aws_config::{defaults, BehaviorVersion, Region};
use aws_sdk_dynamodb::{Client, Error as DynamoDBError};
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::operation::get_item::GetItemError;
use thiserror::Error;
use std::env;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::list_tables::ListTablesError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use tracing::{debug, error};

#[derive(Error, Debug)]
pub enum AWSDynamoDBError {
    #[error("AWS SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),
    #[error("PutItem error: {0}")]
    PutItemError(#[from] SdkError<PutItemError, HttpResponse>),
    #[error("GetItem error: {0}")]
    GetItemError(#[from] SdkError<GetItemError, HttpResponse>),
    #[error("ListTables error: {0}")]
    ListTablesError(#[from] SdkError<ListTablesError, HttpResponse>),
}

pub struct DynamoDBClient {
    client: Client,
}

pub trait DynamoDBClientTrait {
    async fn get_tables(&self) -> Result<Vec<String>, AWSDynamoDBError>;
}

impl DynamoDBClient {
    pub async fn new(profile_name:String , region:String) -> Result<Self, DynamoDBError> {
        let config = defaults(BehaviorVersion::latest())
                .profile_name(profile_name)
                .region(Region::new(region))
                .load()
                .await;
        let client = Client::new(&config);

        Ok(Self {
            client,
        })
    }
}

impl DynamoDBClientTrait for DynamoDBClient {
    async fn get_tables(&self) -> Result<Vec<String>, AWSDynamoDBError> {
        match self.client
            .list_tables()
            .send()
            .await {
            Ok(output) => {
                let table_names = output.table_names.unwrap_or_default();
                debug!("Table names: {:?}", table_names);
                Ok(table_names)
            }
            Err(err) => {
                error!("ListTables error: {:?}", err);
                Err(AWSDynamoDBError::ListTablesError(err)) // Changed this line
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_item() -> Result<(), AWSDynamoDBError> {
        let dynamo_client = DynamoDBClient::new("xalgo_kambi_adapter".to_string(), "eu-west-1".to_string()).await?;
        let table_name = dynamo_client.get_tables().await?;
        assert!(table_name.len() > 0, "No tables found");
        Ok(())
    }
}