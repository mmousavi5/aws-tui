//! DynamoDB client module
//!
//! Provides functionality to interact with AWS DynamoDB service,
//! including listing tables, querying data, and retrieving table metadata.

use aws_config::{BehaviorVersion, Region, defaults};
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::{list_tables::ListTablesError, query::QueryError};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client, Error as DynamoDBError};
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use serde_json::Value;
use thiserror::Error;

/// Errors that can occur when interacting with DynamoDB
#[derive(Error, Debug)]
pub enum DynamoDBClientError {
    /// General AWS SDK error
    #[error("AWS SDK error: {0}")]
    AWSDynamoDBError(#[from] aws_sdk_dynamodb::Error),

    /// Error during ListTables operation
    #[error("ListTables error: {0}")]
    ListTablesError(#[from] SdkError<ListTablesError, HttpResponse>),

    /// Error during Query operation
    #[error("Query error: {0}")]
    QueryError(#[from] SdkError<QueryError, HttpResponse>),

    /// Error during DescribeTable operation
    #[error("DescribeTable error: {0}")]
    DescribeTableError(
        #[from]
        SdkError<aws_sdk_dynamodb::operation::describe_table::DescribeTableError, HttpResponse>,
    ),

    /// No primary key found for table - occurs when table schema is missing or incomplete
    #[error("No primary key found for table")]
    NoPrimaryKeyFound,
}

/// Client for AWS DynamoDB API operations
pub struct DynamoDBClient {
    /// AWS SDK DynamoDB client
    client: Client,
}

impl DynamoDBClient {
    /// Creates a new DynamoDB client with the specified AWS profile and region
    ///
    /// # Parameters
    /// * `profile` - AWS profile name to use for authentication
    /// * `region` - AWS region to connect to
    pub async fn new(profile: String, region: String) -> Result<Self, DynamoDBError> {
        // Configure AWS SDK with profile and region
        let config = defaults(BehaviorVersion::latest())
            .profile_name(profile)
            .region(Region::new(region))
            .load()
            .await;

        Ok(Self {
            client: Client::new(&config),
        })
    }

    /// Retrieves the primary key (partition key) name for a DynamoDB table
    ///
    /// # Parameters
    /// * `table_name` - Name of the table to get the primary key for
    ///
    /// # Returns
    /// The name of the primary key attribute as a String
    pub async fn get_table_primary_key(
        &self,
        table_name: &str,
    ) -> Result<String, DynamoDBClientError> {
        // Get table description from AWS
        let result = self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await?;

        // Extract table schema from response
        let table = result
            .table()
            .ok_or(DynamoDBClientError::NoPrimaryKeyFound)?;

        let key_schema = table.key_schema();
        if key_schema.is_empty() {
            return Err(DynamoDBClientError::NoPrimaryKeyFound);
        }

        // Find the HASH key (partition key) in the key schema
        let primary_key = key_schema
            .iter()
            .find(|k| k.key_type().as_str() == "HASH")
            .ok_or(DynamoDBClientError::NoPrimaryKeyFound)?;

        Ok(primary_key.attribute_name().to_string())
    }

    /// Retrieves the sort key (range key) name for a DynamoDB table if it exists
    ///
    /// # Parameters
    /// * `table_name` - Name of the table to get the sort key for
    ///
    /// # Returns
    /// The name of the sort key attribute as an Option<String>
    pub async fn get_table_sort_key(
        &self,
        table_name: &str,
    ) -> Result<Option<String>, DynamoDBClientError> {
        // Get table description from AWS
        let result = self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await?;

        // Extract table schema from response
        let table = result
            .table()
            .ok_or(DynamoDBClientError::NoPrimaryKeyFound)?;

        let key_schema = table.key_schema();
        
        // Find the RANGE key (sort key) in the key schema
        let sort_key = key_schema
            .iter()
            .find(|k| k.key_type().as_str() == "RANGE")
            .map(|k| k.attribute_name().to_string());

        Ok(sort_key)
    }

        /// Queries a DynamoDB table by its composite key (partition key + optional sort key)
    ///
    /// # Parameters
    /// * `table_name` - Name of the table to query
    /// * `partition_key_value` - Value of the partition key to search for
    /// * `sort_key_value` - Optional value of the sort key for refinement
    ///
    /// # Returns
    /// A vector of JSON strings representing the items found
    pub async fn query_table_composite(
        &self,
        table_name: String,
        partition_key_value: String,
        sort_key_value: Option<String>,
    ) -> Result<Vec<String>, DynamoDBClientError> {
        // First get the primary key name for this table
        let partition_key = self.get_table_primary_key(table_name.as_str()).await?;
        
        // Create attribute value for query parameter
        let pk_attr_value = AttributeValue::S(partition_key_value);
        let mut expression_attribute_values = std::collections::HashMap::new();
        expression_attribute_values.insert(String::from(":pk"), pk_attr_value);
        
        // Create the key condition expression
        let mut key_condition_expr = format!("{} = :pk", partition_key);
        
        // If sort key value is provided, add it to the query
        if let Some(sort_value) = sort_key_value {
            if !sort_value.is_empty() {
                // Get the sort key name
                if let Ok(Some(sort_key)) = self.get_table_sort_key(table_name.as_str()).await {
                    // Only add sort key condition if we found a sort key for this table
                    let sk_attr_value = AttributeValue::S(sort_value);
                    expression_attribute_values.insert(String::from(":sk"), sk_attr_value);
                    
                    // Append sort key condition to expression
                    key_condition_expr = format!("{} AND {} = :sk", key_condition_expr, sort_key);
                }
            }
        }

        // Execute the query with key condition expression
        let output = self
            .client
            .query()
            .table_name(table_name)
            .key_condition_expression(key_condition_expr)
            .set_expression_attribute_values(Some(expression_attribute_values))
            .send()
            .await?;

        // Convert DynamoDB items to JSON strings
        let items = output
            .items()
            .iter()
            .filter_map(|item| {
                // Map each item's attributes to JSON
                let json_value: Value = item
                    .iter()
                    .map(|(k, v)| (k.clone(), DynamoDBClient::attribute_to_json(v)))
                    .collect();

                // Serialize to JSON string, ignoring errors
                serde_json::to_string(&json_value).ok()
            })
            .collect();

        Ok(items)
    }

    /// Lists all DynamoDB tables in the account and region
    ///
    /// # Returns
    /// A vector of table names as Strings
    pub async fn list_tables(&self) -> Result<Vec<String>, DynamoDBClientError> {
        let output = self.client.list_tables().send().await?;
        Ok(output.table_names().to_vec())
    }

    /// Queries a DynamoDB table by its partition key
    ///
    /// # Parameters
    /// * `table_name` - Name of the table to query
    /// * `partition_key_value` - Value of the partition key to search for
    ///
    /// # Returns
    /// A vector of JSON strings representing the items found
    pub async fn query_table(
        &self,
        table_name: String,
        partition_key_value: String,
    ) -> Result<Vec<String>, DynamoDBClientError> {
        // First get the primary key name for this table
        let primary_key = self.get_table_primary_key(table_name.as_str()).await?;

        // Create attribute value for query parameter
        let attr_value = AttributeValue::S(partition_key_value);
        let mut expression_attribute_values = std::collections::HashMap::new();
        expression_attribute_values.insert(String::from(":pk"), attr_value);

        // Execute the query with key condition expression
        let output = self
            .client
            .query()
            .table_name(table_name)
            .key_condition_expression(format!("{} = :pk", primary_key))
            .set_expression_attribute_values(Some(expression_attribute_values))
            .send()
            .await?;

        // Convert DynamoDB items to JSON strings
        let items = output
            .items()
            .iter()
            .filter_map(|item| {
                // Map each item's attributes to JSON
                let json_value: Value = item
                    .iter()
                    .map(|(k, v)| (k.clone(), DynamoDBClient::attribute_to_json(v)))
                    .collect();

                // Serialize to JSON string, ignoring errors
                serde_json::to_string(&json_value).ok()
            })
            .collect();

        Ok(items)
    }

    /// Converts a DynamoDB AttributeValue to a serde JSON Value
    ///
    /// Currently handles String, Number, and Boolean types
    /// Other types are converted to null
    fn attribute_to_json(attr: &AttributeValue) -> Value {
        match attr {
            AttributeValue::S(s) => Value::String(s.clone()),
            AttributeValue::N(n) => {
                // Parse numeric string to float
                if let Ok(num) = n.parse::<f64>() {
                    // Convert to JSON number if valid
                    if let Some(num_value) = serde_json::Number::from_f64(num) {
                        Value::Number(num_value)
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            }
            AttributeValue::Bool(b) => Value::Bool(*b),
            // TODO: Add support for more DynamoDB types (Lists, Maps, Sets, etc.)
            _ => Value::Null,
        }
    }
}
