# aaa - AWS Account Alternator

A Rust CLI tool that simplifies AWS profile management and authentication. It supports both standard AWS credentials and AWS SSO authentication, with an interactive menu for easy profile selection and creation.

## Features

- 🎯 **Interactive Menu**: Choose from existing profiles or create new ones with a user-friendly interface
- ➕ **Profile Creation**: Easily create new SSO profiles with guided prompts
- 🔍 **Profile Discovery**: Automatically reads and lists all AWS profiles from `~/.aws/config`
- 🔐 **SSO Authentication**: Seamlessly handles AWS SSO login flow
- 🔑 **Credential Management**: Works with both SSO profiles and standard credentials from `~/.aws/credentials`
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
  ➕ Add a new profile
     organization1 [SSO]
     organization2 [SSO]
```

**Navigation:**
- Use **arrow keys** (↑/↓) to navigate
- Press **Enter** to select
- Press **Esc** or **Ctrl+C** to cancel

**Options:**
- **Add a new profile**: Create a new SSO profile with guided prompts
- **Select existing profile**: Choose a profile to authenticate and start a shell

### Adding a New Profile

When you select "Add a new profile", you'll be prompted for:

1. **Profile name**: A unique identifier (e.g., `my-org-dev`)
2. **SSO start URL**: Your AWS SSO portal URL (e.g., `https://my-sso-portal.awsapps.com/start`)
3. **SSO region**: The region where your SSO directory is hosted (default: `us-east-1`)
4. **AWS account ID**: The 12-digit AWS account ID
5. **SSO role name**: The role to assume (e.g., `PowerUserAccess`, `Developer`)
6. **Default region**: Default AWS region for this profile (default: `us-east-1`)

After creating the profile, the tool automatically proceeds to authentication!

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

## Requirements

- Rust 1.70 or later (for building from source)
- AWS CLI v2 (required for SSO authentication)
- AWS credentials configured in `~/.aws/config` and/or `~/.aws/credentials`

## Examples

### Example 1: Interactive Menu

```bash
$ aaa
? Select a profile: ›
  ➕ Add a new profile
     org1-dev [SSO]
     org2-prod [SSO]

# Use arrow keys to navigate and Enter to select
```

### Example 2: Create New Profile

```bash
$ aaa
? Select a profile: › ➕ Add a new profile

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

### Example 3: Direct Profile Selection (Skip Menu)

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

### Credentials File Not Found

For standard profiles, ensure:
1. The file `~/.aws/credentials` exists
2. The profile name matches between config and credentials files
3. The credentials contain both `aws_access_key_id` and `aws_secret_access_key`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See [LICENSE](LICENSE) file for details.
