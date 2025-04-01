use aws_config::{defaults, BehaviorVersion, Region};
use aws_sdk_s3::{Client, Error as S3Error};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::ListBucketsError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Error, Debug)]
pub enum AWSS3Error {
    #[error("AWS SDK error: {0}")]
    AWSS3Error(#[from] aws_sdk_s3::Error),
    #[error("ListBuckets error: {0}")]
    ListBucketsError(#[from] SdkError<ListBucketsError, HttpResponse>),
}

pub struct S3Client {
    client: Client,
}

pub trait S3ClientTrait {
    async fn get_buckets(&self) -> Result<Vec<String>, AWSS3Error>;
}

impl S3Client {
    pub async fn new(profile_name: String, region: String) -> Result<Self, S3Error> {
        let config = defaults(BehaviorVersion::latest())
            .profile_name(profile_name)
            .region(Region::new(region))
            .load()
            .await;
        let client = Client::new(&config);

        Ok(Self { client })
    }
}

impl S3ClientTrait for S3Client {
    async fn get_buckets(&self) -> Result<Vec<String>, AWSS3Error> {
        match self.client.list_buckets().send().await {
            Ok(output) => {
                let bucket_names: Vec<String> = output
                    .buckets
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|bucket| bucket.name.clone())
                    .collect();
                debug!("Bucket names: {:?}", bucket_names);
                Ok(bucket_names)
            }
            Err(err) => {
                error!("ListBuckets error: {:?}", err);
                Err(AWSS3Error::ListBucketsError(err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_buckets() -> Result<(), AWSS3Error> {
        let s3_client = S3Client::new("xalgo_kambi_adapter".to_string(), "eu-west-1".to_string()).await?;
        let bucket_names = s3_client.get_buckets().await?;
        assert!(bucket_names.len() > 0, "No buckets found");
        Ok(())
    }
}