mod auth;
mod config;
mod models;
mod profiles;

use anyhow::{anyhow, Context, Result};
use auth::authenticate_and_spawn_shell;
use clap::Parser;
use colored::Colorize;
use config::parse_aws_config;
use inquire::Select;
use models::Profile;
use profiles::{create_new_credentials_profile, create_new_okta_profile, create_new_sso_profile};

#[derive(Parser)]
#[command(name = "aaa")]
#[command(about = "AWS Account Alternator - Manage AWS profiles and SSO authentication")]
#[command(version)]
struct Cli {
    /// Profile name to use (if not specified, shows interactive menu)
    profile: Option<String>,
}

enum MenuAction {
    AddSso,
    AddOkta,
    AddCredentials,
    SelectProfile(String),
}

struct MenuOption {
    label: String,
    action: MenuAction,
}

fn build_menu_options(profiles: &[Profile]) -> Vec<MenuOption> {
    let mut options = vec![
        MenuOption {
            label: "➕ Add a new SSO profile".to_string(),
            action: MenuAction::AddSso,
        },
        MenuOption {
            label: "➕ Add a new Okta profile".to_string(),
            action: MenuAction::AddOkta,
        },
        MenuOption {
            label: "➕ Add a new credentials profile".to_string(),
            action: MenuAction::AddCredentials,
        },
    ];

    for profile in profiles {
        options.push(MenuOption {
            label: format!("   {} [{}]", profile.name, profile.profile_type_label()),
            action: MenuAction::SelectProfile(profile.name.clone()),
        });
    }

    options
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut profiles = parse_aws_config().context("Failed to parse AWS config")?;

    if let Some(profile_name) = cli.profile {
        let profile = profiles
            .iter()
            .find(|p| p.name == profile_name)
            .ok_or_else(|| anyhow!("Profile '{}' not found", profile_name))?;

        authenticate_and_spawn_shell(profile).await?;
        return Ok(());
    }

    loop {
        let menu_options = build_menu_options(&profiles);
        let labels: Vec<String> = menu_options.iter().map(|o| o.label.clone()).collect();

        if profiles.is_empty() {
            println!();
            println!("{}", "No AWS profiles found.".yellow());
            println!("{}", "Let's create your first profile!".cyan());
            println!();
        }

        let selection = Select::new("Select a profile:", labels)
            .with_page_size(10)
            .prompt();

        match selection {
            Ok(choice) => {
                let Some(selected) = menu_options.iter().find(|o| o.label == choice) else {
                    println!();
                    println!("{}", "Invalid profile selection".red());
                    println!();
                    continue;
                };

                match &selected.action {
                    MenuAction::AddSso => match create_new_sso_profile() {
                        Ok(new_profile) => {
                            profiles.push(new_profile.clone());
                            authenticate_and_spawn_shell(&new_profile).await?;
                            break;
                        }
                        Err(e) => {
                            println!();
                            println!("{} {}", "Error creating profile:".red(), e);
                            println!();
                        }
                    },
                    MenuAction::AddOkta => match create_new_okta_profile() {
                        Ok(new_profile) => {
                            profiles.push(new_profile.clone());
                            authenticate_and_spawn_shell(&new_profile).await?;
                            break;
                        }
                        Err(e) => {
                            println!();
                            println!("{} {}", "Error creating profile:".red(), e);
                            println!();
                        }
                    },
                    MenuAction::AddCredentials => match create_new_credentials_profile() {
                        Ok(new_profile) => {
                            profiles.push(new_profile.clone());
                            authenticate_and_spawn_shell(&new_profile).await?;
                            break;
                        }
                        Err(e) => {
                            println!();
                            println!("{} {}", "Error creating profile:".red(), e);
                            println!();
                        }
                    },
                    MenuAction::SelectProfile(profile_name) => {
                        if let Some(profile) = profiles.iter().find(|p| p.name == *profile_name) {
                            authenticate_and_spawn_shell(profile).await?;
                            break;
                        }

                        println!();
                        println!("{} {}", "Profile not found:".red(), profile_name);
                        println!();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_menu_options_maps_profile_names_without_parsing() {
        let profiles = vec![Profile {
            name: "weird[name".to_string(),
            is_sso: true,
            is_okta: false,
            sso_start_url: None,
            sso_region: None,
            sso_account_id: None,
            sso_role_name: None,
            region: None,
            okta_org_domain: None,
            okta_oidc_client_id: None,
            okta_aws_account_federation_app_id: None,
            okta_aws_iam_role: None,
            okta_aws_iam_idp: None,
        }];

        let options = build_menu_options(&profiles);
        let profile_option = options
            .iter()
            .find(|option| matches!(option.action, MenuAction::SelectProfile(_)))
            .unwrap();

        match &profile_option.action {
            MenuAction::SelectProfile(name) => assert_eq!(name, "weird[name"),
            _ => panic!("expected profile selection action"),
        }
    }
}
