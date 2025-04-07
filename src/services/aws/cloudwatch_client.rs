use aws_config::{BehaviorVersion, Region};
use aws_sdk_cloudwatchlogs::error::SdkError;
use aws_sdk_cloudwatchlogs::{Client, Error as CloudWatchLogsError, config};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloudWatchClientError {
    #[error("AWS SDK error: {0}")]
    AwsError(String),

    #[error("Failed to connect with profile: {0}")]
    ConnectionFailed(String),

    #[error("No logs found matching filter")]
    NoItemsFound,
}

// Implement From traits for SDK errors
impl<T, E> From<SdkError<T, E>> for CloudWatchClientError {
    fn from(err: SdkError<T, E>) -> Self {
        CloudWatchClientError::AwsError(err.to_string())
    }
}

pub struct CloudWatchClient {
    client: Client,
}

impl CloudWatchClient {
    pub async fn new(profile: String, region: String) -> Result<Self, CloudWatchClientError> {
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

        // Validate connection by trying to list log groups
        match client.describe_log_groups().send().await {
            Ok(_) => Ok(Self { client }),
            Err(err) => Err(CloudWatchClientError::ConnectionFailed(err.to_string())),
        }
    }

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
    // ...existing code...
    pub async fn list_log_events(
        &self,
        log_group_name: &str,
        filter_pattern: &str,
    ) -> Result<Vec<String>, CloudWatchClientError> {
        // If there's a filter pattern, use filter_log_events, otherwise use get_log_events
        if !filter_pattern.is_empty() {
            let resp = self
                .client
                .filter_log_events()
                .log_group_name(log_group_name)
                .filter_pattern(filter_pattern)
                .limit(100)
                .send()
                .await?;

            let events = resp
                .events()
                .iter()
                .map(|event| {
                    let timestamp = event
                        .timestamp()
                        .map(|ts| {
                            // Convert timestamp to readable format using DateTime instead of NaiveDateTime
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
            // Use get_log_events for simpler listing without filtering
            let resp = self
                .client
                .get_log_events()
                .log_group_name(log_group_name)
                .limit(100)
                .send()
                .await?;

            let events = resp
                .events()
                .iter()
                .map(|event| {
                    let timestamp = event
                        .timestamp()
                        .map(|ts| {
                            // Fix the same issue here
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
    // ...existing code...``
}
