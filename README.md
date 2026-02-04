# aaa - AWS Account Alternator

A Rust CLI tool that simplifies AWS profile management and authentication. It supports standard AWS credentials, AWS SSO authentication, and Okta authentication via okta-aws-cli, with an interactive menu for easy profile selection and creation.

## Features

- 🎯 **Interactive Menu**: Choose from existing profiles or create new ones with a user-friendly interface
- ➕ **Profile Creation**: Easily create new SSO profiles, Okta profiles, or credentials profiles with guided prompts
- 🔍 **Profile Discovery**: Automatically reads and lists all AWS profiles from `~/.aws/config`
- 🔐 **SSO Authentication**: Seamlessly handles AWS SSO login flow
- 🟢 **Okta Authentication**: Supports Okta AWS CLI for authentication via okta-aws-cli tool
- 🔑 **Credential Management**: Works with SSO profiles, Okta profiles, and standard credentials from `~/.aws/credentials`
- 🐚 **Shell Integration**: Spawns a new shell with AWS credentials exported as environment variables
- 🎨 **Colorful Output**: User-friendly colored output for better visibility
- ⚡ **Fast & Reliable**: Built with Rust for performance and safety

## Installation

### From Source

```bash
git clone https://github.com/manoelhc/aaa.git
cd aaa
cargo build --release
# The binary will be at target/release/aaa
```

You can then move the binary to a directory in your PATH:

```bash
sudo cp target/release/aaa /usr/local/bin/
```

## Usage

### Interactive Mode (Recommended)

Run `aaa` without arguments to see an interactive menu:

```bash
aaa
```

You'll see a menu like this:

```
? Select a profile: ›
  ➕ Add a new SSO profile
  ➕ Add a new Okta profile
  ➕ Add a new credentials profile
     organization1 [SSO]
     my-okta-account [Okta]
     my-dev-account [Standard]
```

**Navigation:**
- Use **arrow keys** (↑/↓) to navigate
- Press **Enter** to select
- Press **Esc** or **Ctrl+C** to cancel

**Options:**
- **Add a new SSO profile**: Create a new SSO profile with guided prompts
- **Add a new Okta profile**: Create a new Okta profile for okta-aws-cli authentication
- **Add a new credentials profile**: Create a new profile with AWS access keys
- **Select existing profile**: Choose a profile to authenticate and start a shell

### Adding a New SSO Profile

When you select "Add a new SSO profile", you'll be prompted for:

1. **Profile name**: A unique identifier (e.g., `my-org-dev`)
2. **SSO start URL**: Your AWS SSO portal URL (e.g., `https://my-sso-portal.awsapps.com/start`)
3. **SSO region**: The region where your SSO directory is hosted (default: `us-east-1`)
4. **AWS account ID**: The 12-digit AWS account ID
5. **SSO role name**: The role to assume (e.g., `PowerUserAccess`, `Developer`)
6. **Default region**: Default AWS region for this profile (default: `us-east-1`)

After creating the profile, the tool automatically proceeds to authentication!

### Adding a New Okta Profile

When you select "Add a new Okta profile", you'll be prompted for:

1. **Profile name**: A unique identifier (e.g., `my-org-okta`)
2. **Okta Org Domain**: Full host and domain name (e.g., `my-org.okta.com`)
3. **OIDC Client ID**: The OIDC Native Application Client ID (e.g., `0oa5wyqjk6Wm148fE1d7`)
4. **AWS Account Federation App ID** (optional): ID of the AWS Account Federation integration app (can be empty if OIDC app has `okta.users.read.self` grant)
5. **AWS IAM Role ARN** (optional): AWS IAM Role ARN to assume (e.g., `arn:aws:iam::123456789012:role/MyRole`)
6. **AWS IAM Identity Provider ARN** (optional): AWS IAM IdP ARN (e.g., `arn:aws:iam::123456789012:saml-provider/okta-idp`)
7. **Default region**: Default AWS region for this profile (default: `us-east-1`)

The tool will automatically create:
- Profile configuration in `~/.aws/config`
- Okta configuration in `~/.okta/okta.yaml`

After creating the profile, the tool automatically proceeds to authentication using `okta-aws-cli`!

**Prerequisites for Okta Profiles:**
- Install `okta-aws-cli` (see [okta-aws-cli installation](https://github.com/okta/okta-aws-cli#installation))
- Configure your Okta organization with AWS Federation following [Okta's documentation](https://github.com/okta/okta-aws-cli)

### Adding a New Credentials Profile

When you select "Add a new credentials profile", you'll be prompted for:

1. **Profile name**: A unique identifier (e.g., `my-dev-account`)
2. **AWS Access Key ID**: Your AWS access key ID (e.g., AKIA... for permanent credentials, ASIA... for temporary credentials)
3. **AWS Secret Access Key**: Your AWS secret access key
4. **Default region**: Default AWS region for this profile (default: `us-east-1`)

The credentials are securely stored in `~/.aws/credentials` and the profile configuration is saved to `~/.aws/config`. After creating the profile, the tool automatically proceeds to authentication!

### Direct Profile Selection

To authenticate directly with a specific profile (skip the menu):

```bash
aaa <profile-name>
```

#### For SSO Profiles

When you select an SSO profile, the tool will:
1. Automatically call `aws sso login --profile <profile-name>`
2. Open your browser for authentication
3. Once authenticated, fetch temporary credentials
4. Export credentials as environment variables
5. Start a new shell with these variables

#### For Okta Profiles

When you select an Okta profile, the tool will:
1. Automatically call `okta-aws-cli web` with your profile configuration
2. Open your browser for Okta authentication
3. Once authenticated, fetch temporary AWS credentials
4. Export credentials as environment variables
5. Start a new shell with these variables

#### For Standard Profiles

For standard (non-SSO) profiles, the tool will:
1. Verify credentials exist in `~/.aws/credentials`
2. Fetch and export credentials
3. Start a new shell with these variables

### Environment Variables Set

When you enter the new shell, the following environment variables are automatically set:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_SESSION_TOKEN` (when available)
- `AWS_REGION`
- `AWS_DEFAULT_REGION`
- `AWS_PROFILE`

The shell prompt will be prefixed with `(aws:<profile-name>)` to indicate you're in an AWS session.

### Exiting the AWS Shell

Simply type `exit` or press `Ctrl+D` to return to your original shell.

## Configuration

### AWS Config File (~/.aws/config)

#### SSO Profile Example

```ini
[profile sso-profile]
sso_start_url = https://my-sso-portal.awsapps.com/start
sso_region = us-east-1
sso_account_id = 123456789012
sso_role_name = PowerUserAccess
region = us-west-2
```

#### Okta Profile Example

```ini
[profile okta-profile]
okta_org_domain = my-org.okta.com
okta_oidc_client_id = 0oa5wyqjk6Wm148fE1d7
okta_aws_account_federation_app_id = 0oa9x1rifa2H6Q5d8325
okta_aws_iam_role = arn:aws:iam::123456789012:role/MyRole
okta_aws_iam_idp = arn:aws:iam::123456789012:saml-provider/okta-idp
region = us-east-1
```

#### Standard Profile Example

```ini
[profile standard-profile]
region = eu-west-1
output = json
```

### AWS Credentials File (~/.aws/credentials)

For standard (non-SSO) profiles:

```ini
[standard-profile]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

### Okta Config File (~/.okta/okta.yaml)

For Okta profiles, the tool automatically generates a configuration file compatible with `okta-aws-cli`:

```yaml
---
awscli:
  profiles:
    okta-profile:
      org-domain: "my-org.okta.com"
      oidc-client-id: "0oa5wyqjk6Wm148fE1d7"
      aws-acct-fed-app-id: "0oa9x1rifa2H6Q5d8325"
      aws-iam-role: "arn:aws:iam::123456789012:role/MyRole"
      aws-iam-idp: "arn:aws:iam::123456789012:saml-provider/okta-idp"
```

## Requirements

- Rust 1.70 or later (for building from source)
- AWS CLI v2 (required for SSO authentication)
- `okta-aws-cli` (required for Okta authentication) - [Installation guide](https://github.com/okta/okta-aws-cli#installation)
- AWS credentials configured in `~/.aws/config` and/or `~/.aws/credentials`

## Examples

### Example 1: Interactive Menu

```bash
$ aaa
? Select a profile: ›
  ➕ Add a new SSO profile
  ➕ Add a new Okta profile
  ➕ Add a new credentials profile
     org1-dev [SSO]
     my-okta-account [Okta]
     my-dev-account [Standard]

# Use arrow keys to navigate and Enter to select
```

### Example 2: Create New Okta Profile

```bash
$ aaa
? Select a profile: › ➕ Add a new Okta profile

Create New Okta AWS Profile

? Profile name: › my-company-okta
? Okta Org Domain: › my-company.okta.com
? OIDC Client ID: › 0oa5wyqjk6Wm148fE1d7
? AWS Account Federation App ID (optional): › 0oa9x1rifa2H6Q5d8325
? AWS IAM Role ARN (optional): › arn:aws:iam::123456789012:role/MyRole
? AWS IAM Identity Provider ARN (optional): › arn:aws:iam::123456789012:saml-provider/okta-idp
? Default region: › us-east-1

✓ Profile created successfully!
✓ Created/updated ~/.okta/okta.yaml with profile 'my-company-okta'

Using profile: my-company-okta

This is an Okta profile. Initiating Okta authentication...
Running okta-aws-cli web command...
Note: Your browser may open for authentication
# Browser opens for Okta authentication
✓ Okta authentication successful!
Fetching credentials...

✓ Credentials obtained successfully!

Starting new shell with AWS credentials...
# ... shell starts with credentials
```

### Example 3: Create New SSO Profile

```bash
$ aaa
? Select a profile: › ➕ Add a new SSO profile

Create New AWS SSO Profile

? Profile name: › my-company-dev
? SSO start URL: › https://my-company.awsapps.com/start
? SSO region: › us-east-1
? AWS account ID: › 123456789012
? SSO role name: › Developer
? Default region: › us-east-1

✓ Profile created successfully!

Using profile: my-company-dev

This is an SSO profile. Initiating SSO login...
# Browser opens for authentication
✓ SSO login successful!
Fetching credentials...

✓ Credentials obtained successfully!

Starting new shell with AWS credentials...
# ... shell starts with credentials
```

### Example 4: Create New Credentials Profile

```bash
$ aaa
? Select a profile: › ➕ Add a new credentials profile

Create New AWS Credentials Profile

? Profile name: › my-dev-account
? AWS Access Key ID: › AKIAIOSFODNN7EXAMPLE
? AWS Secret Access Key: › wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
? Default region: › us-east-1

✓ Profile created successfully!

Using profile: my-dev-account

This is a standard profile. Using credentials from ~/.aws/credentials
✓ Credentials found in ~/.aws/credentials
Fetching credentials...

✓ Credentials obtained successfully!

Starting new shell with AWS credentials...
# ... shell starts with credentials
```

### Example 5: Direct Profile Selection (Skip Menu)

```bash
$ aaa org1-dev
Using profile: org1-dev

This is an SSO profile. Initiating SSO login...
Calling AWS SSO login...
# Browser opens for authentication
✓ SSO login successful!
Fetching credentials...

✓ Credentials obtained successfully!

Starting new shell with AWS credentials...
Shell: /bin/bash

Environment variables set:
  - AWS_ACCESS_KEY_ID
  - AWS_SECRET_ACCESS_KEY
  - AWS_SESSION_TOKEN
  - AWS_REGION
  - AWS_PROFILE

Type 'exit' to return to the original shell.

(aws:org1-dev) $ # You're now in a new shell with AWS credentials
(aws:org1-dev) $ aws s3 ls  # This will use the SSO credentials
(aws:org1-dev) $ exit

Returned to original shell.
```

## Troubleshooting

### Profile Not Found

If you see "Profile 'xxx' not found", make sure:
1. The profile exists in `~/.aws/config`
2. The profile name is correct (case-sensitive)
3. For SSO profiles, the section should start with `[profile profile-name]`
4. The default profile should be `[default]` (not `[profile default]`)

### SSO Login Fails

If SSO login fails:
1. Ensure AWS CLI v2 is installed: `aws --version`
2. Check your SSO configuration in `~/.aws/config`
3. Verify your SSO start URL is correct
4. Make sure you have network access to the SSO portal

### Okta Authentication Fails

If Okta authentication fails:
1. Ensure `okta-aws-cli` is installed: `okta-aws-cli --version`
2. Check your Okta configuration in `~/.aws/config` and `~/.okta/okta.yaml`
3. Verify your Okta org domain and OIDC client ID are correct
4. Make sure you have network access to your Okta portal
5. Confirm your OIDC app has the correct grant types enabled (Authorization Code, Device Authorization, Token Exchange)
6. For multiple AWS environments, ensure the OIDC app has the `okta.users.read.self` grant

### Credentials File Not Found

For standard profiles, ensure:
1. The file `~/.aws/credentials` exists
2. The profile name matches between config and credentials files
3. The credentials contain both `aws_access_key_id` and `aws_secret_access_key`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See [LICENSE](LICENSE) file for details.
