//! CloudWatch Logs client module
//! 
//! Provides functionality to interact with AWS CloudWatch Logs service,
//! including listing log groups and retrieving log events with optional filtering.

use aws_config::{BehaviorVersion, Region};
use aws_sdk_cloudwatchlogs::error::SdkError;
use aws_sdk_cloudwatchlogs::{Client, config};
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur when interacting with CloudWatch Logs
#[derive(Error, Debug)]
pub enum CloudWatchClientError {
    /// Error returned from the AWS SDK
    #[error("AWS SDK error: {0}")]
    AwsError(String),

    /// Authentication or connection error with AWS
    #[error("Failed to connect with profile: {0}")]
    ConnectionFailed(String),
}

/// Convert SDK errors to our application-specific error type
impl<T, E> From<SdkError<T, E>> for CloudWatchClientError {
    fn from(err: SdkError<T, E>) -> Self {
        CloudWatchClientError::AwsError(err.to_string())
    }
}

/// Client for AWS CloudWatch Logs API operations
pub struct CloudWatchClient {
    /// AWS SDK CloudWatch Logs client
    client: Client,
}

impl CloudWatchClient {
    /// Creates a new CloudWatch client with the specified AWS profile and region
    ///
    /// Attempts to connect to verify credentials are valid before returning
    pub async fn new(profile: String, region: String) -> Result<Self, CloudWatchClientError> {
        // Configure AWS SDK with profile, region and timeouts
        let config = aws_config::defaults(BehaviorVersion::latest())
            .profile_name(&profile)
            .region(Region::new(region))
            .timeout_config(
                config::timeout::TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(30))
                    .build(),
            )
            .load()
            .await;

        let client = Client::new(&config);

        // Verify credentials by making a simple API call
        match client.describe_log_groups().send().await {
            Ok(_) => Ok(Self { client }),
            Err(err) => Err(CloudWatchClientError::ConnectionFailed(err.to_string())),
        }
    }

    /// Lists available CloudWatch log groups
    ///
    /// Returns a vector of log group names or a friendly message if none found
    pub async fn list_log_groups(&self) -> Result<Vec<String>, CloudWatchClientError> {
        let resp = self.client.describe_log_groups().send().await?;

        let log_groups: Vec<String> = resp
            .log_groups()
            .iter()
            .filter_map(|group| group.log_group_name().map(|name| name.to_string()))
            .collect();

        if log_groups.is_empty() {
            Ok(vec!["No log groups found".to_string()])
        } else {
            Ok(log_groups)
        }
    }

    /// Retrieves log events from a specific log group
    ///
    /// If filter_pattern is provided, uses FilterLogEvents API; otherwise uses GetLogEvents
    /// Returns formatted log entries with timestamps
    pub async fn list_log_events(
        &self,
        log_group_name: &str,
        filter_pattern: &str,
    ) -> Result<Vec<String>, CloudWatchClientError> {
        if !filter_pattern.is_empty() {
            // With filter pattern: use FilterLogEvents for more advanced searching
            let resp = self
                .client
                .filter_log_events()
                .log_group_name(log_group_name)
                .filter_pattern(filter_pattern)
                .limit(100)  // Limit results to prevent overwhelming the UI
                .send()
                .await?;

            let events = resp
                .events()
                .iter()
                .map(|event| {
                    // Format timestamp as readable date/time
                    let timestamp = event
                        .timestamp()
                        .map(|ts| {
                            chrono::DateTime::from_timestamp_millis(ts as i64)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    let message = event.message().unwrap_or_default();

                    format!("[{}] {}", timestamp, message)
                })
                .collect::<Vec<_>>();

            if events.is_empty() {
                Ok(vec!["No matching logs found".to_string()])
            } else {
                Ok(events)
            }
        } else {
            // Without filter: use GetLogEvents for simpler listing
            let resp = self
                .client
                .get_log_events()
                .log_group_name(log_group_name)
                .limit(100)  // Limit results to prevent overwhelming the UI
                .send()
                .await?;

            let events = resp
                .events()
                .iter()
                .map(|event| {
                    // Format timestamp as readable date/time
                    let timestamp = event
                        .timestamp()
                        .map(|ts| {
                            chrono::DateTime::from_timestamp_millis(ts as i64)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    let message = event.message().unwrap_or_default();

                    format!("[{}] {}", timestamp, message)
                })
                .collect::<Vec<_>>();

            if events.is_empty() {
                Ok(vec!["No log events found".to_string()])
            } else {
                Ok(events)
            }
        }
    }
}