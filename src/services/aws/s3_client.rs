use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, Error as S3Error};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::ListBucketsError;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_smithy_runtime_api::client::result::SdkError as SmithySdkError;
use aws_sdk_s3::types::{Bucket, Object};
use serde_json::{json, Value};
use thiserror::Error;
use std::fmt;
use std::time::Duration;

#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("AWS SDK error: {0}")]
    AwsError(String),

    #[error("Failed to connect with profile: {0}")]
    ConnectionFailed(String),

    #[error("No objects found in bucket matching prefix")]
    NoObjectsFound,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// Implement From traits for SDK errors
impl<T, E> From<SdkError<T, E>> for S3ClientError {
    fn from(err: SdkError<T, E>) -> Self {
        S3ClientError::AwsError(err.to_string())
    }
}

pub struct S3Client {
    client: Client,
}

impl S3Client {
    pub async fn new(profile: String, region: String) -> Result<Self, S3ClientError> {
        // Replace from_env() with defaults()
        let config = aws_config::defaults(BehaviorVersion::latest())
            .profile_name(&profile)
            .region(Region::new(region))
            .timeout_config(
                aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(30))
                    .build(),
            )
            .load()
            .await;
    
        let client = Client::new(&config);
    
        // Validate connection by trying to list buckets
        match client.list_buckets().send().await {
            Ok(_) => Ok(Self { client }),
            Err(err) => Err(S3ClientError::ConnectionFailed(err.to_string())),
        }
    }

    pub async fn list_buckets(&self) -> Result<Vec<String>, S3ClientError> {
        let resp = self.client.list_buckets().send().await?;
        
        // Fixed: use unwrap_or_else with an empty vector
        let buckets = resp.buckets();
        let bucket_names = buckets
            .iter()
            .filter_map(|bucket| bucket.name().map(|name| name.to_string()))
            .collect();

        Ok(bucket_names)
    }

    pub async fn list_objects(&self, bucket_name: &str, prefix: &str) -> Result<Vec<String>, S3ClientError> {
        // Build the request with prefix if it's not empty
        let mut request = self.client.list_objects_v2().bucket(bucket_name);
        
        if !prefix.is_empty() {
            request = request.prefix(prefix);
        }

        // Execute the request
        let resp = request.send().await?;
        
        // Check if we have any objects
        // Fixed: use unwrap_or_else with an empty slice
        if resp.contents().is_empty() {
            return Ok(vec!["No objects found".to_string()]);
        }

        // Convert objects to JSON strings
        let objects = resp.contents()
            .iter()
            .map(|obj| {
                let last_modified = obj.last_modified()
                    .map(|dt| dt.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default())
                    .unwrap_or_default();
                
                let size = obj.size().unwrap_or_default();
                let key = obj.key().unwrap_or_default();
                let etag = obj.e_tag().unwrap_or_default();
                
                let json_obj = json!({
                    "key": key,
                    "size": format!("{} bytes", size),
                    "last_modified": last_modified,
                    "etag": etag
                });
                
                serde_json::to_string(&json_obj)
                    .unwrap_or_else(|_| format!("{{\"key\": \"{}\"}}", key))
            })
            .collect();

        Ok(objects)
    }
    
    pub async fn get_object_details(&self, bucket_name: &str, key: &str) -> Result<String, S3ClientError> {
        let resp = self.client
            .head_object()
            .bucket(bucket_name)
            .key(key)
            .send()
            .await?;
        
        // Extract metadata from response
        let content_type = resp.content_type().unwrap_or_default();
        let content_length = resp.content_length().unwrap_or_default();
        let last_modified = resp.last_modified()
            .map(|dt| dt.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default())
            .unwrap_or_default();
        let etag = resp.e_tag().unwrap_or_default();
        
        // Build JSON response with object metadata
        let metadata = json!({
            "key": key,
            "bucket": bucket_name,
            "content_type": content_type,
            "size": format!("{} bytes", content_length),
            "last_modified": last_modified,
            "etag": etag,
            "metadata": resp.metadata()
        });
        
        serde_json::to_string_pretty(&metadata)
            .map_err(|e| S3ClientError::SerializationError(e.to_string()))
    }
}