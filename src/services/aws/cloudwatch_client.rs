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

    /// Parse a time range string (e.g., "15m", "1h", "7d") into milliseconds timestamp
    fn parse_time_range(&self, range: &str, now: chrono::DateTime<chrono::Utc>) -> i64 {
        // Default to 1 hour if parsing fails
        let default_time = now.timestamp_millis() - (60 * 1000);

        // Extract numeric value and unit from the time range string
        let mut numeric = String::new();
        let mut unit = String::new();

        for c in range.chars() {
            if c.is_digit(10) {
                numeric.push(c);
            } else {
                unit.push(c);
            }
        }

        // Parse the numeric part
        let amount: i64 = match numeric.parse() {
            Ok(num) => num,
            Err(_) => return default_time,
        };

        // If amount is 0 or negative, return default
        if amount <= 0 {
            return default_time;
        }

        // Calculate milliseconds based on the unit
        match unit.as_str() {
            "s" => now.timestamp_millis() - (amount * 1000), // seconds
            "m" => now.timestamp_millis() - (amount * 60 * 1000), // minutes
            "h" => now.timestamp_millis() - (amount * 60 * 60 * 1000), // hours
            "d" => now.timestamp_millis() - (amount * 24 * 60 * 60 * 1000), // days
            "w" => now.timestamp_millis() - (amount * 7 * 24 * 60 * 60 * 1000), // weeks
            _ => default_time,                               // Unrecognized unit, return default
        }
    }

    /// Retrieves log events from a specific log group with pagination
    ///
    /// This method fetches all pages of results by following the nextToken
    /// Returns formatted log entries with timestamps
    pub async fn list_log_events(
        &self,
        log_group_name: &str,
        filter_pattern: &str,
        time_range: Option<&str>,
    ) -> Result<Vec<String>, aws_sdk_cloudwatchlogs::Error> {
        let mut start_time = None;
        let mut logs = Vec::new();
        let mut next_token = None;

        // Parse the time range if provided
        let effective_range = time_range.unwrap_or("1m");
        let now = chrono::Utc::now();
        let milliseconds = self.parse_time_range(effective_range, now);
        start_time = Some(milliseconds);

        // Continue fetching pages until there are no more results
        loop {
            // Build the filter log events request
            let mut request = self
                .client
                .filter_log_events()
                .log_group_name(log_group_name);

            if !filter_pattern.is_empty() {
                request = request.filter_pattern(filter_pattern);
            }

            if let Some(time) = start_time {
                request = request.start_time(time);
            }

            // Add the next token if we have one from a previous page
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            // Execute the request
            let response = request.send().await?;

            // Process log events from this page
            let events = response.events();
            for event in events {
                if let Some(message) = event.message() {
                    logs.push(message.to_string());
                }
            }

            // Get the next token for pagination
            next_token = response.next_token().map(String::from);

            // Break the loop if there's no next token
            if next_token.is_none() {
                break;
            }
        }

        // Add helpful message when no logs found
        if logs.is_empty() {
            if !filter_pattern.is_empty() {
                logs.push(format!(
                    "No logs matching filter '{}' found in the time range",
                    filter_pattern
                ));
            } else if let Some(range) = time_range {
                logs.push(format!(
                    "No logs found in the specified time range ({})",
                    range
                ));
            } else {
                logs.push("No logs found".to_string());
            }
        }

        Ok(logs)
    }
}
