use aws_config::{defaults, BehaviorVersion, Region};
use aws_sdk_s3::{Client, Error as S3Error};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::ListBucketsError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("AWS SDK error: {0}")]
    AWSS3Error(#[from] aws_sdk_s3::Error),
    #[error("ListBuckets error: {0}")]
    ListBucketsError(#[from] SdkError<ListBucketsError, HttpResponse>),
}

pub struct S3Client {
    client: Client,
}

impl S3Client {
    pub async fn new(profile: String, region: String) -> Result<Self, S3Error> {
        let config = defaults(BehaviorVersion::latest())
            .profile_name(profile)
            .region(Region::new(region))
            .load()
            .await;
        Ok(Self { 
            client: Client::new(&config) 
        })
    }

    pub async fn list_buckets(&self) -> Result<Vec<String>, S3ClientError> {
        let output = self.client.list_buckets().send().await?;
        Ok(output.buckets()
            .iter()
            .filter_map(|bucket| bucket.name().map(String::from))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::profile::Profile;

    #[tokio::test]
    async fn test_list_buckets() {
        let profile = "xalgo_kambi_adapter".to_string();
        let region = "eu-west-1".to_string();
        let client = S3Client::new(profile, region).await.unwrap();
        let buckets = client.list_buckets().await.unwrap();
        assert!(buckets.len() > 0);
    }
}