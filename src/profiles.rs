use crate::config::{
    create_okta_yaml, parse_aws_config, profile_exists_in_credentials, save_credentials_to_file,
    save_profile_to_config, validate_profile_name,
};
use crate::models::Profile;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use inquire::{Password, Text};

fn prompt_profile_name(help_message: &str) -> Result<String> {
    let profile_name = Text::new("Profile name:")
        .with_help_message(help_message)
        .prompt()
        .context("Failed to get profile name")?
        .trim()
        .to_string();

    validate_profile_name(&profile_name)?;

    let existing_profiles = parse_aws_config()?;
    if existing_profiles.iter().any(|p| p.name == profile_name) {
        return Err(anyhow!("Profile '{}' already exists", profile_name));
    }

    Ok(profile_name)
}

pub fn create_new_sso_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New AWS SSO Profile".bold().green());
    println!();

    let profile_name = prompt_profile_name("A unique name for this profile (e.g., my-org-dev)")?;

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
        sso_start_url: Some(sso_start_url),
        sso_region: Some(sso_region),
        sso_account_id: Some(sso_account_id),
        sso_role_name: Some(sso_role_name),
        region: Some(region),
        okta_org_domain: None,
        okta_oidc_client_id: None,
        okta_aws_account_federation_app_id: None,
        okta_aws_iam_role: None,
        okta_aws_iam_idp: None,
    };

    save_profile_to_config(&profile)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}

pub fn create_new_okta_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New Okta AWS Profile".bold().green());
    println!();

    let profile_name = prompt_profile_name("A unique name for this profile (e.g., my-org-okta)")?;

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
        .context("Failed to prompt for AWS Account Federation App ID")?;

    let okta_aws_iam_role = Text::new("AWS IAM Role ARN (optional):")
        .with_help_message(
            "AWS IAM Role ARN to assume (e.g., arn:aws:iam::123456789012:role/MyRole)",
        )
        .prompt()
        .context("Failed to prompt for AWS IAM role")?;

    let okta_aws_iam_idp = Text::new("AWS IAM Identity Provider ARN (optional):")
        .with_help_message(
            "AWS IAM IdP ARN (e.g., arn:aws:iam::123456789012:saml-provider/okta-idp)",
        )
        .prompt()
        .context("Failed to prompt for AWS IAM IdP")?;

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
        region: Some(region),
        okta_org_domain: Some(okta_org_domain),
        okta_oidc_client_id: Some(okta_oidc_client_id),
        okta_aws_account_federation_app_id: if okta_aws_account_federation_app_id.is_empty() {
            None
        } else {
            Some(okta_aws_account_federation_app_id)
        },
        okta_aws_iam_role: if okta_aws_iam_role.is_empty() {
            None
        } else {
            Some(okta_aws_iam_role)
        },
        okta_aws_iam_idp: if okta_aws_iam_idp.is_empty() {
            None
        } else {
            Some(okta_aws_iam_idp)
        },
    };

    save_profile_to_config(&profile)?;
    create_okta_yaml(&profile)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}

pub fn create_new_credentials_profile() -> Result<Profile> {
    println!();
    println!("{}", "Create New AWS Credentials Profile".bold().green());
    println!();

    let profile_name =
        prompt_profile_name("A unique name for this profile (e.g., my-dev-account)")?;

    if profile_exists_in_credentials(&profile_name)? {
        return Err(anyhow!(
            "Profile '{}' already exists in credentials file",
            profile_name
        ));
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

    if !access_key_id.chars().all(|c| c.is_alphanumeric()) {
        return Err(anyhow!(
            "Access Key ID should only contain alphanumeric characters"
        ));
    }

    let secret_access_key = Password::new("AWS Secret Access Key:")
        .with_help_message("Your AWS secret access key")
        .without_confirmation()
        .prompt()
        .context("Failed to get secret access key")?
        .trim()
        .to_string();

    if secret_access_key.is_empty() {
        return Err(anyhow!("Secret Access Key cannot be empty"));
    }

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
        region: Some(region),
        okta_org_domain: None,
        okta_oidc_client_id: None,
        okta_aws_account_federation_app_id: None,
        okta_aws_iam_role: None,
        okta_aws_iam_idp: None,
    };

    save_profile_to_config(&profile)?;
    save_credentials_to_file(&profile_name, &access_key_id, &secret_access_key)?;

    println!();
    println!("{}", "✓ Profile created successfully!".green().bold());
    println!();

    Ok(profile)
}
