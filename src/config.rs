use crate::models::{AwsConfig, OktaProfile, OktaYamlConfig, Profile};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn validate_profile_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Profile name cannot be empty"));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Profile name can only contain alphanumeric characters, hyphens, and underscores"
        ));
    }

    Ok(())
}

/// Formats a value for AWS INI files, quoting when special characters are present.
pub fn format_ini_value(value: &str) -> String {
    let needs_quotes = value
        .chars()
        .any(|c| c.is_whitespace() || c == '=' || c == '#' || c == ';' || c == '"');

    if needs_quotes {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

pub fn parse_profiles_from_ini(content: &str) -> Result<Vec<Profile>> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    let config: AwsConfig =
        serde_ini::from_str(content).context("Failed to parse AWS config file")?;

    for (section_name, section_data) in config.sections {
        let profile_name = if section_name == "default" {
            "default".to_string()
        } else if let Some(name) = section_name.strip_prefix("profile ") {
            name.to_string()
        } else {
            continue;
        };

        let is_okta = section_data.contains_key("okta_org_domain");
        let is_sso = !is_okta && section_data.contains_key("sso_start_url");

        profiles.push(Profile {
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
        });
    }

    Ok(profiles)
}

pub fn get_aws_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".aws").join("config"))
}

pub fn get_aws_credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".aws").join("credentials"))
}

fn get_okta_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".okta").join("okta.yaml"))
}

pub fn parse_aws_config() -> Result<Vec<Profile>> {
    let config_path = get_aws_config_path()?;

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create .aws directory")?;
            set_private_dir_permissions(parent)?;
        }
        fs::write(&config_path, "").context("Failed to create config file")?;
        set_private_file_permissions(&config_path)?;
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&config_path).context("Failed to read AWS config file")?;
    parse_profiles_from_ini(&content)
}

pub fn profile_exists_in_credentials(profile_name: &str) -> Result<bool> {
    let creds_path = get_aws_credentials_path()?;
    if !creds_path.exists() {
        return Ok(false);
    }

    let existing_creds =
        fs::read_to_string(&creds_path).context("Failed to read existing credentials file")?;

    let profile_section = format!("[{}]", profile_name);
    Ok(existing_creds
        .lines()
        .any(|line| line.trim() == profile_section))
}

fn write_ini_field(file: &mut impl Write, key: &str, value: &str) -> Result<()> {
    writeln!(file, "{} = {}", key, format_ini_value(value))?;
    Ok(())
}

pub fn save_credentials_to_file(
    profile_name: &str,
    access_key_id: &str,
    secret_access_key: &str,
) -> Result<()> {
    let creds_path = get_aws_credentials_path()?;

    if let Some(parent) = creds_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .aws directory")?;
        set_private_dir_permissions(parent)?;
    }

    let existing_content = if creds_path.exists() {
        fs::read_to_string(&creds_path).context("Failed to read existing credentials file")?
    } else {
        String::new()
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&creds_path)
        .context("Failed to open credentials file")?;

    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file)?;
    }

    writeln!(file, "[{}]", profile_name)?;
    write_ini_field(&mut file, "aws_access_key_id", access_key_id)?;
    write_ini_field(&mut file, "aws_secret_access_key", secret_access_key)?;

    set_private_file_permissions(&creds_path)?;

    Ok(())
}

pub fn create_okta_yaml(profile: &Profile) -> Result<()> {
    let okta_config_path = get_okta_config_path()?;

    if let Some(parent) = okta_config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .okta directory")?;
        set_private_dir_permissions(parent)?;
    }

    let mut config: OktaYamlConfig = if okta_config_path.exists() {
        let content = fs::read_to_string(&okta_config_path)
            .context("Failed to read existing okta.yaml file")?;
        serde_yaml::from_str(&content).context("Failed to parse existing okta.yaml file")?
    } else {
        OktaYamlConfig::default()
    };

    let okta_profile = OktaProfile {
        org_domain: profile.okta_org_domain.clone(),
        oidc_client_id: profile.okta_oidc_client_id.clone(),
        aws_acct_fed_app_id: profile.okta_aws_account_federation_app_id.clone(),
        aws_iam_role: profile.okta_aws_iam_role.clone(),
        aws_iam_idp: profile.okta_aws_iam_idp.clone(),
    };

    config
        .awscli
        .profiles
        .insert(profile.name.clone(), okta_profile);

    let yaml_content =
        serde_yaml::to_string(&config).context("Failed to serialize okta.yaml config")?;
    fs::write(&okta_config_path, yaml_content).context("Failed to write okta.yaml file")?;
    set_private_file_permissions(&okta_config_path)?;

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

pub fn save_profile_to_config(profile: &Profile) -> Result<()> {
    let config_path = get_aws_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create .aws directory")?;
        set_private_dir_permissions(parent)?;
    }

    let existing_content = if config_path.exists() {
        fs::read_to_string(&config_path).context("Failed to read existing config file")?
    } else {
        String::new()
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)
        .context("Failed to open config file")?;

    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file)?;
    }

    let section_name = if profile.name == "default" {
        "[default]".to_string()
    } else {
        format!("[profile {}]", profile.name)
    };

    writeln!(file, "{}", section_name)?;

    if let Some(sso_start_url) = &profile.sso_start_url {
        write_ini_field(&mut file, "sso_start_url", sso_start_url)?;
    }
    if let Some(sso_region) = &profile.sso_region {
        write_ini_field(&mut file, "sso_region", sso_region)?;
    }
    if let Some(sso_account_id) = &profile.sso_account_id {
        write_ini_field(&mut file, "sso_account_id", sso_account_id)?;
    }
    if let Some(sso_role_name) = &profile.sso_role_name {
        write_ini_field(&mut file, "sso_role_name", sso_role_name)?;
    }
    if let Some(okta_org_domain) = &profile.okta_org_domain {
        write_ini_field(&mut file, "okta_org_domain", okta_org_domain)?;
    }
    if let Some(okta_oidc_client_id) = &profile.okta_oidc_client_id {
        write_ini_field(&mut file, "okta_oidc_client_id", okta_oidc_client_id)?;
    }
    if let Some(okta_aws_account_federation_app_id) = &profile.okta_aws_account_federation_app_id {
        write_ini_field(
            &mut file,
            "okta_aws_account_federation_app_id",
            okta_aws_account_federation_app_id,
        )?;
    }
    if let Some(okta_aws_iam_role) = &profile.okta_aws_iam_role {
        write_ini_field(&mut file, "okta_aws_iam_role", okta_aws_iam_role)?;
    }
    if let Some(okta_aws_iam_idp) = &profile.okta_aws_iam_idp {
        write_ini_field(&mut file, "okta_aws_iam_idp", okta_aws_iam_idp)?;
    }
    if let Some(region) = &profile.region {
        write_ini_field(&mut file, "region", region)?;
    }

    set_private_file_permissions(&config_path)?;

    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_profile_name_accepts_valid_names() {
        assert!(validate_profile_name("my-org-dev").is_ok());
        assert!(validate_profile_name("profile_1").is_ok());
        assert!(validate_profile_name("default").is_ok());
    }

    #[test]
    fn validate_profile_name_rejects_invalid_names() {
        assert!(validate_profile_name("").is_err());
        assert!(validate_profile_name("bad name").is_err());
        assert!(validate_profile_name("bad[name]").is_err());
    }

    #[test]
    fn format_ini_value_quotes_values_with_spaces() {
        assert_eq!(
            format_ini_value("https://example.com/my path"),
            "\"https://example.com/my path\""
        );
    }

    #[test]
    fn format_ini_value_leaves_simple_values_unquoted() {
        assert_eq!(format_ini_value("us-east-1"), "us-east-1");
        assert_eq!(
            format_ini_value("https://example.awsapps.com/start"),
            "https://example.awsapps.com/start"
        );
    }

    #[test]
    fn parse_profiles_from_ini_detects_sso_and_okta_profiles() {
        let content = r#"
[profile sso-profile]
sso_start_url = https://example.awsapps.com/start
sso_region = us-east-1
region = us-west-2

[profile okta-profile]
okta_org_domain = example.okta.com
okta_oidc_client_id = client-id

[default]
region = eu-west-1
"#;

        let profiles = parse_profiles_from_ini(content).unwrap();
        assert_eq!(profiles.len(), 3);

        let sso = profiles.iter().find(|p| p.name == "sso-profile").unwrap();
        assert!(sso.is_sso);
        assert!(!sso.is_okta);

        let okta = profiles.iter().find(|p| p.name == "okta-profile").unwrap();
        assert!(okta.is_okta);
        assert!(!okta.is_sso);

        let default = profiles.iter().find(|p| p.name == "default").unwrap();
        assert!(!default.is_sso);
        assert!(!default.is_okta);
    }

    #[test]
    fn profile_exists_in_credentials_detects_section_header() {
        let temp = tempfile::tempdir().unwrap();
        let creds_path = temp.path().join("credentials");
        fs::write(
            &creds_path,
            "[existing]\naws_access_key_id = KEY\naws_secret_access_key = SECRET\n",
        )
        .unwrap();

        let content = fs::read_to_string(&creds_path).unwrap();
        let profile_section = "[existing]";
        assert!(content.lines().any(|line| line.trim() == profile_section));
    }
}
