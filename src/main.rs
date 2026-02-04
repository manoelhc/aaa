use anyhow::{anyhow, Context, Result};
use aws_credential_types::provider::ProvideCredentials;
use clap::Parser;
use colored::Colorize;
use inquire::{Select, Text};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "aaa")]
#[command(about = "AWS Account Alternator - Manage AWS profiles and SSO authentication")]
#[command(version)]
struct Cli {
    /// Profile name to use (if not specified, shows interactive menu)
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AwsConfig {
    #[serde(flatten)]
    sections: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Clone)]
struct Profile {
    name: String,
    is_sso: bool,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    sso_account_id: Option<String>,
    sso_role_name: Option<String>,
    region: Option<String>,
}

fn get_aws_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".aws").join("config"))
}

fn get_aws_credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".aws").join("credentials"))
}

fn parse_aws_config() -> Result<Vec<Profile>> {
    let config_path = get_aws_config_path()?;
    
    if !config_path.exists() {
        // Create empty config if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create .aws directory")?;
        }
        fs::write(&config_path, "").context("Failed to create config file")?;
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read AWS config file")?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    let config: AwsConfig = serde_ini::from_str(&content)
        .context("Failed to parse AWS config file")?;

    for (section_name, section_data) in config.sections {
        let profile_name = if section_name == "default" {
            "default".to_string()
        } else if let Some(name) = section_name.strip_prefix("profile ") {
            name.to_string()
        } else {
            continue;
        };

        let is_sso = section_data.contains_key("sso_start_url");

        let profile = Profile {
            name: profile_name,
            is_sso,
            sso_start_url: section_data.get("sso_start_url").cloned(),
            sso_region: section_data.get("sso_region").cloned(),
            sso_account_id: section_data.get("sso_account_id").cloned(),
            sso_role_name: section_data.get("sso_role_name").cloned(),
            region: section_data.get("region").cloned(),
        };

        profiles.push(profile);
    }

    Ok(profiles)
}

fn create_new_sso_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New AWS SSO Profile".bold().green());
    println!();

    let profile_name = Text::new("Profile name:")
        .with_help_message("A unique name for this profile (e.g., my-org-dev)")
        .prompt()
        .context("Failed to get profile name")?
        .trim()
        .to_string();

    if profile_name.is_empty() {
        return Err(anyhow!("Profile name cannot be empty"));
    }

    // Check if profile already exists
    let existing_profiles = parse_aws_config()?;
    if existing_profiles.iter().any(|p| p.name == profile_name) {
        return Err(anyhow!("Profile '{}' already exists", profile_name));
    }

    let sso_start_url = Text::new("SSO start URL:")
        .with_help_message("The AWS SSO portal URL (e.g., https://my-sso-portal.awsapps.com/start)")
        .prompt()
        .context("Failed to get SSO start URL")?;

    let sso_region = Text::new("SSO region:")
        .with_default("us-east-1")
        .with_help_message("The AWS region where your SSO directory is hosted")
        .prompt()
        .context("Failed to get SSO region")?;

    let sso_account_id = Text::new("AWS account ID:")
        .with_help_message("The 12-digit AWS account ID")
        .prompt()
        .context("Failed to get account ID")?;

    let sso_role_name = Text::new("SSO role name:")
        .with_help_message("The role name to assume (e.g., PowerUserAccess)")
        .prompt()
        .context("Failed to get role name")?;

    let region = Text::new("Default region:")
        .with_default("us-east-1")
        .with_help_message("Default AWS region for this profile")
        .prompt()
        .context("Failed to get region")?;

    let profile = Profile {
        name: profile_name.clone(),
        is_sso: true,
        sso_start_url: Some(sso_start_url.clone()),
        sso_region: Some(sso_region.clone()),
        sso_account_id: Some(sso_account_id.clone()),
        sso_role_name: Some(sso_role_name.clone()),
        region: Some(region.clone()),
    };

    // Write profile to config file
    save_profile_to_config(&profile)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}

fn create_new_credentials_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New AWS Credentials Profile".bold().green());
    println!();

    let profile_name = Text::new("Profile name:")
        .with_help_message("A unique name for this profile (e.g., my-dev-account)")
        .prompt()
        .context("Failed to get profile name")?
        .trim()
        .to_string();

    if profile_name.is_empty() {
        return Err(anyhow!("Profile name cannot be empty"));
    }

    // Validate profile name - no special characters except hyphens and underscores
    if !profile_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow!("Profile name can only contain alphanumeric characters, hyphens, and underscores"));
    }

    // Check if profile already exists
    let existing_profiles = parse_aws_config()?;
    if existing_profiles.iter().any(|p| p.name == profile_name) {
        return Err(anyhow!("Profile '{}' already exists", profile_name));
    }

    let access_key_id = Text::new("AWS Access Key ID:")
        .with_help_message("Your AWS access key ID (starts with AKIA)")
        .prompt()
        .context("Failed to get access key ID")?
        .trim()
        .to_string();

    if access_key_id.is_empty() {
        return Err(anyhow!("Access Key ID cannot be empty"));
    }

    // Validate access key ID format
    if !access_key_id.chars().all(|c| c.is_alphanumeric()) {
        return Err(anyhow!("Access Key ID should only contain alphanumeric characters"));
    }

    let secret_access_key = Text::new("AWS Secret Access Key:")
        .with_help_message("Your AWS secret access key")
        .prompt()
        .context("Failed to get secret access key")?
        .trim()
        .to_string();

    if secret_access_key.is_empty() {
        return Err(anyhow!("Secret Access Key cannot be empty"));
    }

    // Validate secret access key format (base64 characters)
    if !secret_access_key.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=') {
        return Err(anyhow!("Secret Access Key contains invalid characters"));
    }

    let region = Text::new("Default region:")
        .with_default("us-east-1")
        .with_help_message("Default AWS region for this profile")
        .prompt()
        .context("Failed to get region")?;

    // Validate region format
    if !region.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(anyhow!("Region should only contain alphanumeric characters and hyphens"));
    }

    let profile = Profile {
        name: profile_name.clone(),
        is_sso: false,
        sso_start_url: None,
        sso_region: None,
        sso_account_id: None,
        sso_role_name: None,
        region: Some(region.clone()),
    };

    // Write profile to config file
    save_profile_to_config(&profile)?;

    // Write credentials to credentials file
    save_credentials_to_file(&profile_name, &access_key_id, &secret_access_key)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}

fn save_credentials_to_file(profile_name: &str, access_key_id: &str, secret_access_key: &str) -> Result<()> {
    let creds_path = get_aws_credentials_path()?;
    
    // Ensure the directory exists
    if let Some(parent) = creds_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .aws directory")?;
    }

    // Read existing content or create empty
    let existing_content = if creds_path.exists() {
        fs::read_to_string(&creds_path)
            .context("Failed to read existing credentials file")?
    } else {
        String::new()
    };

    // Check if profile already exists in credentials file
    if existing_content.contains(&format!("[{}]", profile_name)) {
        return Err(anyhow!("Profile '{}' already exists in credentials file", profile_name));
    }

    // Append new credentials
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&creds_path)
        .context("Failed to open credentials file")?;

    // Add newline if file is not empty
    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file)?;
    }

    // Write credentials section
    writeln!(file, "[{}]", profile_name)?;
    writeln!(file, "aws_access_key_id = {}", access_key_id)?;
    writeln!(file, "aws_secret_access_key = {}", secret_access_key)?;

    Ok(())
}

fn save_profile_to_config(profile: &Profile) -> Result<()> {
    let config_path = get_aws_config_path()?;
    
    // Ensure the directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .aws directory")?;
    }

    // Read existing content or create empty
    let existing_content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .context("Failed to read existing config file")?
    } else {
        String::new()
    };

    // Append new profile
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)
        .context("Failed to open config file")?;

    // Add newline if file is not empty
    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file)?;
    }

    // Write profile section
    let section_name = if profile.name == "default" {
        "[default]".to_string()
    } else {
        format!("[profile {}]", profile.name)
    };

    writeln!(file, "{}", section_name)?;
    
    if let Some(sso_start_url) = &profile.sso_start_url {
        writeln!(file, "sso_start_url = {}", sso_start_url)?;
    }
    if let Some(sso_region) = &profile.sso_region {
        writeln!(file, "sso_region = {}", sso_region)?;
    }
    if let Some(sso_account_id) = &profile.sso_account_id {
        writeln!(file, "sso_account_id = {}", sso_account_id)?;
    }
    if let Some(sso_role_name) = &profile.sso_role_name {
        writeln!(file, "sso_role_name = {}", sso_role_name)?;
    }
    if let Some(region) = &profile.region {
        writeln!(file, "region = {}", region)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut profiles = parse_aws_config()
        .context("Failed to parse AWS config")?;

    // If profile specified via command line, use it directly
    if let Some(profile_name) = cli.profile {
        let profile = profiles.iter()
            .find(|p| p.name == profile_name)
            .ok_or_else(|| anyhow!("Profile '{}' not found", profile_name))?;

        authenticate_and_spawn_shell(profile).await?;
        return Ok(());
    }

    // Interactive mode: show menu
    loop {
        let mut options: Vec<String> = Vec::new();
        options.push("➕ Add a new SSO profile".to_string());
        options.push("➕ Add a new credentials profile".to_string());
        
        for profile in &profiles {
            let profile_type = if profile.is_sso { "SSO" } else { "Standard" };
            options.push(format!("   {} [{}]", profile.name, profile_type));
        }

        if profiles.is_empty() {
            println!();
            println!("{}", "No AWS profiles found.".yellow());
            println!("{}", "Let's create your first profile!".cyan());
            println!();
        }

        let selection = Select::new("Select a profile:", options)
            .with_page_size(10)
            .prompt();

        match selection {
            Ok(choice) => {
                if choice.starts_with("➕ Add a new SSO profile") {
                    // Create new SSO profile
                    match create_new_sso_profile() {
                        Ok(new_profile) => {
                            profiles.push(new_profile.clone());
                            authenticate_and_spawn_shell(&new_profile).await?;
                            break;
                        }
                        Err(e) => {
                            println!();
                            println!("{} {}", "Error creating profile:".red(), e);
                            println!();
                            continue;
                        }
                    }
                } else if choice.starts_with("➕ Add a new credentials profile") {
                    // Create new credentials profile
                    match create_new_credentials_profile() {
                        Ok(new_profile) => {
                            profiles.push(new_profile.clone());
                            authenticate_and_spawn_shell(&new_profile).await?;
                            break;
                        }
                        Err(e) => {
                            println!();
                            println!("{} {}", "Error creating profile:".red(), e);
                            println!();
                            continue;
                        }
                    }
                } else {
                    // Extract profile name from selection (remove leading spaces and type indicator)
                    let profile_name = choice
                        .trim()
                        .split('[')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    if profile_name.is_empty() {
                        println!();
                        println!("{}", "Invalid profile selection".red());
                        println!();
                        continue;
                    }

                    if let Some(profile) = profiles.iter().find(|p| p.name == profile_name) {
                        authenticate_and_spawn_shell(profile).await?;
                        break;
                    } else {
                        println!();
                        println!("{} {}", "Profile not found:".red(), profile_name);
                        println!();
                        continue;
                    }
                }
            }
            Err(_) => {
                println!();
                println!("{}", "Cancelled.".dimmed());
                return Ok(());
            }
        }
    }

    Ok(())
}

async fn authenticate_and_spawn_shell(profile: &Profile) -> Result<()> {
    println!();
    println!("{} {}", "Using profile:".bold(), profile.name.green().bold());
    println!();

    // Set AWS_PROFILE environment variable
    env::set_var("AWS_PROFILE", &profile.name);

    if profile.is_sso {
        println!("{}", "This is an SSO profile. Initiating SSO login...".yellow());
        sso_login(profile).await?;
    } else {
        println!("{}", "This is a standard profile. Using credentials from ~/.aws/credentials".blue());
        verify_credentials(profile)?;
    }

    // Get credentials and export to environment
    let credentials = get_credentials(profile).await?;
    
    println!();
    println!("{}", "✓ Credentials obtained successfully!".green().bold());
    println!();

    // Spawn new shell with credentials
    spawn_shell_with_credentials(profile, credentials)?;

    Ok(())
}

async fn sso_login(profile: &Profile) -> Result<()> {
    println!("Calling AWS SSO login...");
    
    let output = Command::new("aws")
        .args(["sso", "login", "--profile", &profile.name])
        .status()
        .context("Failed to execute 'aws sso login'")?;

    if !output.success() {
        return Err(anyhow!("SSO login failed"));
    }

    println!("{}", "✓ SSO login successful!".green());
    Ok(())
}

fn verify_credentials(profile: &Profile) -> Result<()> {
    let creds_path = get_aws_credentials_path()?;
    
    if !creds_path.exists() {
        return Err(anyhow!(
            "Credentials file not found at {:?}. Please configure your AWS credentials.",
            creds_path
        ));
    }

    let content = fs::read_to_string(&creds_path)
        .context("Failed to read AWS credentials file")?;

    let config: AwsConfig = serde_ini::from_str(&content)
        .context("Failed to parse AWS credentials file")?;

    if !config.sections.contains_key(&profile.name) {
        return Err(anyhow!(
            "Profile '{}' not found in credentials file",
            profile.name
        ));
    }

    println!("{}", "✓ Credentials found in ~/.aws/credentials".green());
    Ok(())
}

async fn get_credentials(profile: &Profile) -> Result<HashMap<String, String>> {
    use aws_config::BehaviorVersion;
    
    println!("Fetching credentials...");
    
    // Load AWS config with the specified profile
    let config = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(&profile.name)
        .load()
        .await;

    let credentials = config
        .credentials_provider()
        .ok_or_else(|| anyhow!("No credentials provider available"))?
        .provide_credentials()
        .await
        .context("Failed to retrieve credentials")?;

    let mut creds_map = HashMap::new();
    creds_map.insert("AWS_ACCESS_KEY_ID".to_string(), credentials.access_key_id().to_string());
    creds_map.insert("AWS_SECRET_ACCESS_KEY".to_string(), credentials.secret_access_key().to_string());
    
    if let Some(token) = credentials.session_token() {
        creds_map.insert("AWS_SESSION_TOKEN".to_string(), token.to_string());
    }

    if let Some(region) = &profile.region {
        creds_map.insert("AWS_REGION".to_string(), region.clone());
        creds_map.insert("AWS_DEFAULT_REGION".to_string(), region.clone());
    }

    creds_map.insert("AWS_PROFILE".to_string(), profile.name.clone());

    Ok(creds_map)
}

fn spawn_shell_with_credentials(profile: &Profile, credentials: HashMap<String, String>) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    
    println!("{}", "Starting new shell with AWS credentials...".cyan().bold());
    println!("{}", format!("Shell: {}", shell).dimmed());
    println!();
    println!("{}", "Environment variables set:".dimmed());
    println!("{}", "  - AWS_ACCESS_KEY_ID".dimmed());
    println!("{}", "  - AWS_SECRET_ACCESS_KEY".dimmed());
    if credentials.contains_key("AWS_SESSION_TOKEN") {
        println!("{}", "  - AWS_SESSION_TOKEN".dimmed());
    }
    println!("{}", "  - AWS_REGION".dimmed());
    println!("{}", "  - AWS_PROFILE".dimmed());
    println!();
    println!("{}", "Type 'exit' to return to the original shell.".to_string().yellow());
    println!();

    let mut command = Command::new(&shell);
    
    // Set AWS credentials as environment variables
    for (key, value) in credentials {
        command.env(key, value);
    }

    // Preserve PATH and other important environment variables
    if let Ok(path) = env::var("PATH") {
        command.env("PATH", path);
    }
    if let Ok(home) = env::var("HOME") {
        command.env("HOME", home);
    }
    if let Ok(user) = env::var("USER") {
        command.env("USER", user);
    }

    // Update PS1 to show we're in an AWS session
    let ps1_prefix = format!("(aws:{}) ", profile.name);
    if let Ok(current_ps1) = env::var("PS1") {
        command.env("PS1", format!("{}{}", ps1_prefix, current_ps1));
    } else {
        command.env("PS1", format!("{}\\$ ", ps1_prefix));
    }

    let status = command
        .status()
        .context("Failed to spawn shell")?;

    if !status.success() {
        return Err(anyhow!("Shell exited with error"));
    }

    println!();
    println!("{}", "Returned to original shell.".green());

    Ok(())
}
