use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub(crate) struct AwsConfig {
    #[serde(flatten)]
    pub sections: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OktaYamlConfig {
    pub awscli: OktaAwsCli,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OktaAwsCli {
    #[serde(default)]
    pub profiles: HashMap<String, OktaProfile>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OktaProfile {
    #[serde(rename = "org-domain", skip_serializing_if = "Option::is_none")]
    pub org_domain: Option<String>,
    #[serde(rename = "oidc-client-id", skip_serializing_if = "Option::is_none")]
    pub oidc_client_id: Option<String>,
    #[serde(
        rename = "aws-acct-fed-app-id",
        skip_serializing_if = "Option::is_none"
    )]
    pub aws_acct_fed_app_id: Option<String>,
    #[serde(rename = "aws-iam-role", skip_serializing_if = "Option::is_none")]
    pub aws_iam_role: Option<String>,
    #[serde(rename = "aws-iam-idp", skip_serializing_if = "Option::is_none")]
    pub aws_iam_idp: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Profile {
    pub name: String,
    pub is_sso: bool,
    pub is_okta: bool,
    pub sso_start_url: Option<String>,
    pub sso_region: Option<String>,
    pub sso_account_id: Option<String>,
    pub sso_role_name: Option<String>,
    pub region: Option<String>,
    pub okta_org_domain: Option<String>,
    pub okta_oidc_client_id: Option<String>,
    pub okta_aws_account_federation_app_id: Option<String>,
    pub okta_aws_iam_role: Option<String>,
    pub okta_aws_iam_idp: Option<String>,
}

impl Profile {
    pub fn profile_type_label(&self) -> &'static str {
        if self.is_okta {
            "Okta"
        } else if self.is_sso {
            "SSO"
        } else {
            "Standard"
        }
    }
}
