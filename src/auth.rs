use crate::config::get_aws_credentials_path;
use crate::models::{AwsConfig, Profile};
use anyhow::{anyhow, Context, Result};
use aws_credential_types::provider::ProvideCredentials;
use colored::Colorize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::Command;

pub async fn authenticate_and_spawn_shell(profile: &Profile) -> Result<()> {
    println!();
    println!(
        "{} {}",
        "Using profile:".bold(),
        profile.name.green().bold()
    );
    println!();

    // SAFETY: Setting AWS_PROFILE before invoking external AWS tooling and loading credentials.
    unsafe {
        env::set_var("AWS_PROFILE", &profile.name);
    }

    if profile.is_okta {
        println!(
            "{}",
            "This is an Okta profile. Initiating Okta authentication...".yellow()
        );
        okta_login(profile)?;
    } else if profile.is_sso {
        println!(
            "{}",
            "This is an SSO profile. Initiating SSO login...".yellow()
        );
        sso_login(profile)?;
    } else {
        println!(
            "{}",
            "This is a standard profile. Using credentials from ~/.aws/credentials".blue()
        );
        verify_credentials(profile)?;
    }

    let credentials = get_credentials(profile).await?;

    println!();
    println!("{}", "✓ Credentials obtained successfully!".green().bold());
    println!();

    spawn_shell_with_credentials(profile, credentials)?;

    Ok(())
}

fn sso_login(profile: &Profile) -> Result<()> {
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

fn okta_login(profile: &Profile) -> Result<()> {
    println!("Calling okta-aws-cli for authentication...");

    let mut cmd = Command::new("okta-aws-cli");
    cmd.arg("web");

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

    if let Some(ref app_id) = profile.okta_aws_account_federation_app_id {
        cmd.args(["--aws-acct-fed-app-id", app_id]);
    }

    if let Some(ref iam_role) = profile.okta_aws_iam_role {
        cmd.args(["--aws-iam-role", iam_role]);
    }

    if let Some(ref iam_idp) = profile.okta_aws_iam_idp {
        cmd.args(["--aws-iam-idp", iam_idp]);
    }

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

    for (key, value) in credentials {
        command.env(key, value);
    }

    if let Ok(path) = env::var("PATH") {
        command.env("PATH", path);
    }
    if let Ok(home) = env::var("HOME") {
        command.env("HOME", home);
    }
    if let Ok(user) = env::var("USER") {
        command.env("USER", user);
    }

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
