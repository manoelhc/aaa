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
    is_okta: bool,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    sso_account_id: Option<String>,
    sso_role_name: Option<String>,
    region: Option<String>,
    // Okta-specific fields
    okta_org_domain: Option<String>,
    okta_oidc_client_id: Option<String>,
    okta_aws_account_federation_app_id: Option<String>,
    okta_aws_iam_role: Option<String>,
    okta_aws_iam_idp: Option<String>,
}

fn get_aws_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".aws").join("config"))
}

fn get_aws_credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
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

    let content = fs::read_to_string(&config_path).context("Failed to read AWS config file")?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    let config: AwsConfig =
        serde_ini::from_str(&content).context("Failed to parse AWS config file")?;

    for (section_name, section_data) in config.sections {
        let profile_name = if section_name == "default" {
            "default".to_string()
        } else if let Some(name) = section_name.strip_prefix("profile ") {
            name.to_string()
        } else {
            continue;
        };

        // Determine profile type: Okta, SSO, or Standard
        let is_okta = section_data.contains_key("okta_org_domain");
        let is_sso = section_data.contains_key("sso_start_url");

        let profile = Profile {
            name: profile_name,
            is_sso,
            is_okta,
            sso_start_url: section_data.get("sso_start_url").cloned(),
            sso_region: section_data.get("sso_region").cloned(),
            sso_account_id: section_data.get("sso_account_id").cloned(),
            sso_role_name: section_data.get("sso_role_name").cloned(),
            region: section_data.get("region").cloned(),
            okta_org_domain: section_data.get("okta_org_domain").cloned(),
            okta_oidc_client_id: section_data.get("okta_oidc_client_id").cloned(),
            okta_aws_account_federation_app_id: section_data
                .get("okta_aws_account_federation_app_id")
                .cloned(),
            okta_aws_iam_role: section_data.get("okta_aws_iam_role").cloned(),
            okta_aws_iam_idp: section_data.get("okta_aws_iam_idp").cloned(),
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
        is_okta: false,
        sso_start_url: Some(sso_start_url.clone()),
        sso_region: Some(sso_region.clone()),
        sso_account_id: Some(sso_account_id.clone()),
        sso_role_name: Some(sso_role_name.clone()),
        region: Some(region.clone()),
        okta_org_domain: None,
        okta_oidc_client_id: None,
        okta_aws_account_federation_app_id: None,
        okta_aws_iam_role: None,
        okta_aws_iam_idp: None,
    };

    // Write profile to config file
    save_profile_to_config(&profile)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}

fn create_new_okta_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New Okta AWS Profile".bold().green());
    println!();

    let profile_name = Text::new("Profile name:")
        .with_help_message("A unique name for this profile (e.g., my-org-okta)")
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

    let okta_org_domain = Text::new("Okta Org Domain:")
        .with_help_message("Full host and domain name of the Okta org (e.g., my-org.okta.com)")
        .prompt()
        .context("Failed to get Okta org domain")?;

    let okta_oidc_client_id = Text::new("OIDC Client ID:")
        .with_help_message("The OIDC Native Application Client ID (e.g., 0oa5wyqjk6Wm148fE1d7)")
        .prompt()
        .context("Failed to get OIDC client ID")?;

    let okta_aws_account_federation_app_id = Text::new("AWS Account Federation App ID (optional):")
        .with_help_message("ID of the AWS Account Federation integration app (can be empty if OIDC app has okta.users.read.self grant)")
        .prompt()
        .context("Failed to get AWS Account Federation App ID")?;

    let okta_aws_iam_role = Text::new("AWS IAM Role ARN (optional):")
        .with_help_message(
            "AWS IAM Role ARN to assume (e.g., arn:aws:iam::123456789012:role/MyRole)",
        )
        .prompt()
        .context("Failed to get AWS IAM role")?;

    let okta_aws_iam_idp = Text::new("AWS IAM Identity Provider ARN (optional):")
        .with_help_message(
            "AWS IAM IdP ARN (e.g., arn:aws:iam::123456789012:saml-provider/okta-idp)",
        )
        .prompt()
        .context("Failed to get AWS IAM IdP")?;

    let region = Text::new("Default region:")
        .with_default("us-east-1")
        .with_help_message("Default AWS region for this profile")
        .prompt()
        .context("Failed to get region")?;

    let profile = Profile {
        name: profile_name.clone(),
        is_sso: false,
        is_okta: true,
        sso_start_url: None,
        sso_region: None,
        sso_account_id: None,
        sso_role_name: None,
        region: Some(region.clone()),
        okta_org_domain: Some(okta_org_domain.clone()),
        okta_oidc_client_id: Some(okta_oidc_client_id.clone()),
        okta_aws_account_federation_app_id: if okta_aws_account_federation_app_id.is_empty() {
            None
        } else {
            Some(okta_aws_account_federation_app_id.clone())
        },
        okta_aws_iam_role: if okta_aws_iam_role.is_empty() {
            None
        } else {
            Some(okta_aws_iam_role.clone())
        },
        okta_aws_iam_idp: if okta_aws_iam_idp.is_empty() {
            None
        } else {
            Some(okta_aws_iam_idp.clone())
        },
    };

    // Write profile to config file
    save_profile_to_config(&profile)?;

    // Create okta.yaml configuration
    create_okta_yaml(&profile)?;

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
    if !profile_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Profile name can only contain alphanumeric characters, hyphens, and underscores"
        ));
    }

    // Check if profile already exists in config
    let existing_profiles = parse_aws_config()?;
    if existing_profiles.iter().any(|p| p.name == profile_name) {
        return Err(anyhow!(
            "Profile '{}' already exists in config",
            profile_name
        ));
    }

    // Check if profile already exists in credentials file
    let creds_path = get_aws_credentials_path()?;
    if creds_path.exists() {
        let existing_creds =
            fs::read_to_string(&creds_path).context("Failed to read existing credentials file")?;

        // Use a more precise check: look for profile name as a complete section header
        let profile_section = format!("[{}]", profile_name);
        for line in existing_creds.lines() {
            if line.trim() == profile_section {
                return Err(anyhow!(
                    "Profile '{}' already exists in credentials file",
                    profile_name
                ));
            }
        }
    }

    let access_key_id = Text::new("AWS Access Key ID:")
        .with_help_message("Your AWS access key ID (e.g., AKIA..., ASIA...)")
        .prompt()
        .context("Failed to get access key ID")?
        .trim()
        .to_string();

    if access_key_id.is_empty() {
        return Err(anyhow!("Access Key ID cannot be empty"));
    }

    // Basic validation: Access keys should be alphanumeric
    // We don't enforce strict format as AWS supports multiple types (AKIA, ASIA, etc.)
    if !access_key_id.chars().all(|c| c.is_alphanumeric()) {
        return Err(anyhow!(
            "Access Key ID should only contain alphanumeric characters"
        ));
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
    if !secret_access_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(anyhow!("Secret Access Key contains invalid characters"));
    }

    let region = Text::new("Default region:")
        .with_default("us-east-1")
        .with_help_message("Default AWS region for this profile")
        .prompt()
        .context("Failed to get region")?;

    // Validate region format
    if !region.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(anyhow!(
            "Region should only contain alphanumeric characters and hyphens"
        ));
    }

    let profile = Profile {
        name: profile_name.clone(),
        is_sso: false,
        is_okta: false,
        sso_start_url: None,
        sso_region: None,
        sso_account_id: None,
        sso_role_name: None,
        region: Some(region.clone()),
        okta_org_domain: None,
        okta_oidc_client_id: None,
        okta_aws_account_federation_app_id: None,
        okta_aws_iam_role: None,
        okta_aws_iam_idp: None,
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

fn save_credentials_to_file(
    profile_name: &str,
    access_key_id: &str,
    secret_access_key: &str,
) -> Result<()> {
    let creds_path = get_aws_credentials_path()?;

    // Ensure the directory exists
    if let Some(parent) = creds_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .aws directory")?;
    }

    // Read existing content or create empty
    let existing_content = if creds_path.exists() {
        fs::read_to_string(&creds_path).context("Failed to read existing credentials file")?
    } else {
        String::new()
    };

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

fn get_okta_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".okta").join("okta.yaml"))
}

fn create_okta_yaml(profile: &Profile) -> Result<()> {
    let okta_config_path = get_okta_config_path()?;

    // Ensure the directory exists
    if let Some(parent) = okta_config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .okta directory")?;
    }

    // Read existing content or create empty structure
    let mut yaml_content = if okta_config_path.exists() {
        fs::read_to_string(&okta_config_path).context("Failed to read existing okta.yaml file")?
    } else {
        "---\nawscli:\n  profiles:\n".to_string()
    };

    // Check if we need to initialize the structure
    if !yaml_content.contains("awscli:") {
        yaml_content = "---\nawscli:\n  profiles:\n".to_string();
    } else if !yaml_content.contains("profiles:") {
        yaml_content = yaml_content.replace("awscli:", "awscli:\n  profiles:");
    }

    // Build profile configuration
    let mut profile_config = format!("    {}:\n", profile.name);

    if let Some(ref org_domain) = profile.okta_org_domain {
        profile_config.push_str(&format!("      org-domain: \"{}\"\n", org_domain));
    }
    if let Some(ref oidc_client_id) = profile.okta_oidc_client_id {
        profile_config.push_str(&format!("      oidc-client-id: \"{}\"\n", oidc_client_id));
    }
    if let Some(ref app_id) = profile.okta_aws_account_federation_app_id {
        profile_config.push_str(&format!("      aws-acct-fed-app-id: \"{}\"\n", app_id));
    }
    if let Some(ref iam_role) = profile.okta_aws_iam_role {
        profile_config.push_str(&format!("      aws-iam-role: \"{}\"\n", iam_role));
    }
    if let Some(ref iam_idp) = profile.okta_aws_iam_idp {
        profile_config.push_str(&format!("      aws-iam-idp: \"{}\"\n", iam_idp));
    }

    // Append the new profile configuration
    yaml_content.push_str(&profile_config);

    // Write the updated content
    fs::write(&okta_config_path, yaml_content).context("Failed to write okta.yaml file")?;

    println!();
    println!(
        "{}",
        format!(
            "✓ Created/updated ~/.okta/okta.yaml with profile '{}'",
            profile.name
        )
        .green()
    );

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
        fs::read_to_string(&config_path).context("Failed to read existing config file")?
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

    // Write SSO fields if present
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

    // Write Okta fields if present
    if let Some(okta_org_domain) = &profile.okta_org_domain {
        writeln!(file, "okta_org_domain = {}", okta_org_domain)?;
    }
    if let Some(okta_oidc_client_id) = &profile.okta_oidc_client_id {
        writeln!(file, "okta_oidc_client_id = {}", okta_oidc_client_id)?;
    }
    if let Some(okta_aws_account_federation_app_id) = &profile.okta_aws_account_federation_app_id {
        writeln!(
            file,
            "okta_aws_account_federation_app_id = {}",
            okta_aws_account_federation_app_id
        )?;
    }
    if let Some(okta_aws_iam_role) = &profile.okta_aws_iam_role {
        writeln!(file, "okta_aws_iam_role = {}", okta_aws_iam_role)?;
    }
    if let Some(okta_aws_iam_idp) = &profile.okta_aws_iam_idp {
        writeln!(file, "okta_aws_iam_idp = {}", okta_aws_iam_idp)?;
    }

    // Write common region field
    if let Some(region) = &profile.region {
        writeln!(file, "region = {}", region)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut profiles = parse_aws_config().context("Failed to parse AWS config")?;

    // If profile specified via command line, use it directly
    if let Some(profile_name) = cli.profile {
        let profile = profiles
            .iter()
            .find(|p| p.name == profile_name)
            .ok_or_else(|| anyhow!("Profile '{}' not found", profile_name))?;

        authenticate_and_spawn_shell(profile).await?;
        return Ok(());
    }

    // Interactive mode: show menu
    loop {
        let mut options: Vec<String> = Vec::new();
        options.push("➕ Add a new SSO profile".to_string());
        options.push("➕ Add a new Okta profile".to_string());
        options.push("➕ Add a new credentials profile".to_string());

        for profile in &profiles {
            let profile_type = if profile.is_okta {
                "Okta"
            } else if profile.is_sso {
                "SSO"
            } else {
                "Standard"
            };
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
                } else if choice.starts_with("➕ Add a new Okta profile") {
                    // Create new Okta profile
                    match create_new_okta_profile() {
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
    println!(
        "{} {}",
        "Using profile:".bold(),
        profile.name.green().bold()
    );
    println!();

    // Set AWS_PROFILE environment variable
    env::set_var("AWS_PROFILE", &profile.name);

    if profile.is_okta {
        println!(
            "{}",
            "This is an Okta profile. Initiating Okta authentication...".yellow()
        );
        okta_login(profile).await?;
    } else if profile.is_sso {
        println!(
            "{}",
            "This is an SSO profile. Initiating SSO login...".yellow()
        );
        sso_login(profile).await?;
    } else {
        println!(
            "{}",
            "This is a standard profile. Using credentials from ~/.aws/credentials".blue()
        );
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

async fn okta_login(profile: &Profile) -> Result<()> {
    println!("Calling okta-aws-cli for authentication...");

    // Build the okta-aws-cli command
    let mut cmd = Command::new("okta-aws-cli");
    cmd.arg("web");

    // Add required parameters
    if let Some(ref org_domain) = profile.okta_org_domain {
        cmd.args(["--org-domain", org_domain]);
    } else {
        return Err(anyhow!("Okta org domain is required but not configured"));
    }

    if let Some(ref oidc_client_id) = profile.okta_oidc_client_id {
        cmd.args(["--oidc-client-id", oidc_client_id]);
    } else {
        return Err(anyhow!("OIDC client ID is required but not configured"));
    }

    // Add optional parameters
    if let Some(ref app_id) = profile.okta_aws_account_federation_app_id {
        cmd.args(["--aws-acct-fed-app-id", app_id]);
    }

    if let Some(ref iam_role) = profile.okta_aws_iam_role {
        cmd.args(["--aws-iam-role", iam_role]);
    }

    if let Some(ref iam_idp) = profile.okta_aws_iam_idp {
        cmd.args(["--aws-iam-idp", iam_idp]);
    }

    // Set output format to AWS credentials file
    cmd.args(["--format", "aws-credentials"]);
    cmd.args(["--profile", &profile.name]);
    cmd.arg("--write-aws-credentials");

    println!("Running okta-aws-cli web command...");
    println!(
        "{}",
        "Note: Your browser may open for authentication".dimmed()
    );

    let output = cmd.status().context(
        "Failed to execute 'okta-aws-cli'. Make sure okta-aws-cli is installed and in your PATH.",
    )?;

    if !output.success() {
        return Err(anyhow!("Okta authentication failed"));
    }

    println!("{}", "✓ Okta authentication successful!".green());
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

    let content = fs::read_to_string(&creds_path).context("Failed to read AWS credentials file")?;

    let config: AwsConfig =
        serde_ini::from_str(&content).context("Failed to parse AWS credentials file")?;

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
    creds_map.insert(
        "AWS_ACCESS_KEY_ID".to_string(),
        credentials.access_key_id().to_string(),
    );
    creds_map.insert(
        "AWS_SECRET_ACCESS_KEY".to_string(),
        credentials.secret_access_key().to_string(),
    );

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

fn spawn_shell_with_credentials(
    profile: &Profile,
    credentials: HashMap<String, String>,
) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    println!(
        "{}",
        "Starting new shell with AWS credentials...".cyan().bold()
    );
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
    println!(
        "{}",
        "Type 'exit' to return to the original shell.".yellow()
    );
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

    let status = command.status().context("Failed to spawn shell")?;

    if !status.success() {
        return Err(anyhow!("Shell exited with error"));
    }

    println!();
    println!("{}", "Returned to original shell.".green());

    Ok(())
}
