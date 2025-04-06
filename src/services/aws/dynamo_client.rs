use aws_config::{BehaviorVersion, Region, defaults};
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::{list_tables::ListTablesError, query::QueryError};
use aws_sdk_dynamodb::{Client, Error as DynamoDBError};
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use aws_sdk_dynamodb::types::AttributeValue;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DynamoDBClientError {
    #[error("AWS SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),
    #[error("ListTables error: {0}")]
    ListTablesError(#[from] SdkError<ListTablesError, HttpResponse>),
    #[error("Query error: {0}")]
    QueryError(#[from] SdkError<QueryError, HttpResponse>),
    #[error("DescribeTable error: {0}")]
    DescribeTableError(#[from] SdkError<aws_sdk_dynamodb::operation::describe_table::DescribeTableError, HttpResponse>),
    #[error("No primary key found for table")]
    NoPrimaryKeyFound,
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
            client: Client::new(&config),
        })
    }

    pub async fn get_table_primary_key(&self, table_name: &str) -> Result<String, DynamoDBClientError> {
        let result = self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await?;
    
        let table = result.table().ok_or(DynamoDBClientError::NoPrimaryKeyFound)?;
        // Check if key_schema is empty instead of using ok_or
        let key_schema = table.key_schema();
        if key_schema.is_empty() {
            return Err(DynamoDBClientError::NoPrimaryKeyFound);
        }
    
        // Find the HASH key (primary key) from the key schema
        let primary_key = key_schema
            .iter()
            .find(|k| k.key_type().as_str() == "HASH")
            .ok_or(DynamoDBClientError::NoPrimaryKeyFound)?;
    
        Ok(primary_key.attribute_name().to_string())
    }

    pub async fn list_tables(&self) -> Result<Vec<String>, DynamoDBClientError> {
        let output = self.client.list_tables().send().await?;
        Ok(output.table_names().to_vec())
    }

    pub async fn query_table(
        &self,
        table_name: String,
        partition_key_value: String,
    ) -> Result<Vec<String>, DynamoDBClientError> {
        let primary_key = self.get_table_primary_key(table_name.as_str()).await?;
        let attr_value = AttributeValue::S(partition_key_value);
        let mut expression_attribute_values = std::collections::HashMap::new();
        expression_attribute_values.insert(String::from(":pk"), attr_value);
    
        let output = self
            .client
            .query()
            .table_name(table_name)
            .key_condition_expression(format!("{} = :pk", primary_key))
            .set_expression_attribute_values(Some(expression_attribute_values))
            .send()
            .await?;
    
        let items = output.items().iter()
            .filter_map(|item| {
                let json_value: Value = item.iter()
                    .map(|(k, v)| (k.clone(), DynamoDBClient::attribute_to_json(v)))
                    .collect();
                serde_json::to_string(&json_value).ok()
            })
            .collect();
    
        Ok(items)
    }
    
    // Helper function to convert AttributeValue to serde_json::Value
    fn attribute_to_json(attr: &AttributeValue) -> Value {
        match attr {
            AttributeValue::S(s) => Value::String(s.clone()),
            AttributeValue::N(n) => {
                if let Ok(num) = n.parse::<f64>() {
                    if let Some(num_value) = serde_json::Number::from_f64(num) {
                        Value::Number(num_value)
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            },
            AttributeValue::Bool(b) => Value::Bool(*b),
            // Add more cases as needed
            _ => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::profile::Profile;

    #[tokio::test]
    async fn test_list_tables() {
        let profile = "xalgo_kambi_adapter".to_string();
        let region = "eu-west-1".to_string();
        let client = DynamoDBClient::new(profile, region).await.unwrap();
        let tables = client.list_tables().await.unwrap();
        assert!(tables.len() > 0);
    }

    #[tokio::test]
    async fn test_query_table() {
        let profile = "xalgo_kambi_adapter".to_string();
        let region = "eu-west-1".to_string();
        let client = DynamoDBClient::new(profile, region).await.unwrap();
        let table_name = "test1"; // Replace with your actual table name
        let query_resulat = client
            .query_table(table_name.to_string(), "1".to_string())
            .await
            .unwrap();
        assert!(query_resulat.len() == 3); // Replace with actual assertions based on your table data

    }
}