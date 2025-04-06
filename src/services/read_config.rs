use config::{Config, File, FileFormat};
use std::collections::HashMap;
use std::error::Error;

/// Reads the AWS configuration file and extracts profile names.
pub fn get_aws_profiles() -> Result<Vec<String>, Box<dyn Error>> {
    // Specify the path to the AWS config file
    let config_path = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".aws/config");

    // Load the INI file using the `config` crate
    let settings = Config::builder()
        .add_source(File::new(config_path.to_str().unwrap(), FileFormat::Ini))
        .build()?;

    // Deserialize the file into a HashMap
    let config_map: HashMap<String, HashMap<String, String>> = settings.try_deserialize()?;

    // Collect profile names into a Vec
    let mut profiles: Vec<String> = config_map
        .keys()
        .filter_map(|section| section.strip_prefix("profile ").map(String::from))
        .collect();
    profiles.sort();

    Ok(profiles)
}
