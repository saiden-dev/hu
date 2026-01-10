[![crates.io](https://img.shields.io/badge/crates.io-0.1.0--pre17-E9A240)](https://crates.io/crates/hu/0.1.0-pre19)

# hu

Dev workflow CLI for EKS pods, Jira, GitHub Actions, and AWS.

## Install

```bash
cargo install hu
```

## Usage

```bash
# EKS pods
hu eks                    # List pods
hu eks -p 1               # Shell into pod #1
hu eks -e prod -t api     # List api pods on prod
hu eks --log              # Tail logs from pods

# EC2 instances
hu ec2                    # List instances
hu ec2 -e prod            # Filter by environment
hu ec2 -t bastion -p 1    # Connect via SSM

# AWS
hu aws whoami             # Show identity
hu aws discover           # Scan profiles & capabilities

# Jira
hu jira mine              # My assigned issues
hu jira show PROJ-123     # Show issue details

# GitHub
hu gh runs                # Workflow runs
hu gh runs --ok           # Only successful runs
```

## Config

Settings file: `~/.config/hu/settings.toml`

```toml
[aws]
region = "us-east-1"

[kubernetes]
namespace = "cms"
pod_type = "web"

[env.prod]
cluster = "prod-eks"
emoji = "🔴"
```
