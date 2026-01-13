use anyhow::{anyhow, Context, Result};
use aws_credential_types::provider::ProvideCredentials;
use clap::Parser;
use colored::Colorize;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "aaa")]
#[command(about = "AWS Account Alternator - Manage AWS profiles and SSO authentication")]
struct Cli {
    /// Profile name to use (if not specified, shows all profiles)
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AwsConfig {
    #[serde(flatten)]
    sections: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug)]
struct Profile {
    name: String,
    is_sso: bool,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    sso_account_id: Option<String>,
    sso_role_name: Option<String>,
    region: Option<String>,
}

fn get_aws_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".aws")
        .join("config")
}

fn get_aws_credentials_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".aws")
        .join("credentials")
}

fn parse_aws_config() -> Result<Vec<Profile>> {
    let config_path = get_aws_config_path();
    
    if !config_path.exists() {
        return Err(anyhow!("AWS config file not found at {:?}", config_path));
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read AWS config file")?;

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

fn list_profiles(profiles: &[Profile]) {
    println!("{}", "Available AWS Profiles:".bold().green());
    println!();
    
    for profile in profiles {
        let profile_type = if profile.is_sso {
            "SSO".yellow()
        } else {
            "Standard".blue()
        };
        
        println!("  {} [{}]", profile.name.bold(), profile_type);
        
        if let Some(region) = &profile.region {
            println!("    Region: {}", region);
        }
        
        if profile.is_sso {
            if let Some(url) = &profile.sso_start_url {
                println!("    SSO URL: {}", url);
            }
            if let Some(account) = &profile.sso_account_id {
                println!("    Account: {}", account);
            }
            if let Some(role) = &profile.sso_role_name {
                println!("    Role: {}", role);
            }
        }
        
        println!();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let profiles = parse_aws_config()
        .context("Failed to parse AWS config")?;

    if profiles.is_empty() {
        println!("{}", "No AWS profiles found in ~/.aws/config".red());
        return Ok(());
    }

    // If no profile specified, list all profiles
    if cli.profile.is_none() {
        list_profiles(&profiles);
        println!("{}", "Usage: aaa <profile-name>".dimmed());
        return Ok(());
    }

    let profile_name = cli.profile.unwrap();
    let profile = profiles.iter()
        .find(|p| p.name == profile_name)
        .ok_or_else(|| anyhow!("Profile '{}' not found", profile_name))?;

    println!("{} {}", "Using profile:".bold(), profile.name.green().bold());
    println!();

    // Set AWS_PROFILE environment variable
    env::set_var("AWS_PROFILE", &profile.name);

    if profile.is_sso {
        println!("{}", "This is an SSO profile. Initiating SSO login...".yellow());
        sso_login(&profile).await?;
    } else {
        println!("{}", "This is a standard profile. Using credentials from ~/.aws/credentials".blue());
        verify_credentials(&profile)?;
    }

    // Get credentials and export to environment
    let credentials = get_credentials(&profile).await?;
    
    println!();
    println!("{}", "✓ Credentials obtained successfully!".green().bold());
    println!();

    // Spawn new shell with credentials
    spawn_shell_with_credentials(&profile, credentials)?;

    Ok(())
}

async fn sso_login(profile: &Profile) -> Result<()> {
    println!("Calling AWS SSO login...");
    
    let output = Command::new("aws")
        .args(&["sso", "login", "--profile", &profile.name])
        .status()
        .context("Failed to execute 'aws sso login'")?;

    if !output.success() {
        return Err(anyhow!("SSO login failed"));
    }

    println!("{}", "✓ SSO login successful!".green());
    Ok(())
}

fn verify_credentials(profile: &Profile) -> Result<()> {
    let creds_path = get_aws_credentials_path();
    
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
    println!("{}", "  - AWS_SESSION_TOKEN".dimmed());
    println!("{}", "  - AWS_REGION".dimmed());
    println!("{}", "  - AWS_PROFILE".dimmed());
    println!();
    println!("{}", format!("Type 'exit' to return to the original shell.").yellow());
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
