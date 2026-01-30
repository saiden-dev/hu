# To Implement

Legacy commands from the Node.js version that need to be ported to Rust.

## Priority 1: Data & Analytics

Commands for Claude Code session data analysis.

### hu data
| Command | Description |
|---------|-------------|
| `hu data sync` | Sync Claude data to local SQLite database |
| `hu data config` | Show current configuration |
| `hu data session list` | List sessions |
| `hu data session read <id>` | Read session by ID |
| `hu data stats` | Show usage statistics |
| `hu data todos` | Show pending todos |
| `hu data search <query>` | Search messages |
| `hu data tools` | Show tool usage statistics |
| `hu data errors` | Extract errors from debug logs |
| `hu data pricing` | Show pricing analysis |
| `hu data branches` | Show sessions grouped by git branch |

## Priority 2: GitHub Extensions

### hu gh (missing subcommands)
| Command | Description |
|---------|-------------|
| `hu gh config` | Show GitHub configuration |
| `hu gh fix` | Analyze CI failures, output investigation context |
| `hu gh open <pr>` | Open PR in browser |
| `hu gh stats` | Show workflow run duration statistics |

## Priority 3: Jira Extensions

### hu jira (missing subcommands)
| Command | Description |
|---------|-------------|
| `hu jira analyze <ticket>` | Analyze ticket and investigate related code |
| `hu jira branch <ticket>` | Create git branch from ticket |
| `hu jira status` | Generate daily status report |
| `hu jira config` | Show Jira configuration |
| `hu jira check` | Check sprint tickets without descriptions |
| `hu jira open <ticket>` | Open ticket in browser |
| `hu jira prs <ticket>` | Show PRs linked to a ticket |

## Priority 4: Settings Management

### hu settings
| Command | Description |
|---------|-------------|
| `hu settings show` | Show settings from all scopes |
| `hu settings edit` | Edit settings file in $EDITOR |
| `hu settings managed` | View/edit managed settings |
| `hu settings allow <pattern>` | Add a permission rule |
| `hu settings path` | Show paths to settings files |

## Priority 5: Disk Analysis (macOS)

### hu disk
| Command | Description |
|---------|-------------|
| `hu disk overview` | APFS volume usage |
| `hu disk home` | Home directory breakdown |
| `hu disk library` | ~/Library breakdown |
| `hu disk hogs` | Check common space hogs |
| `hu disk cleanup` | Cleanup suggestions |
| `hu disk models` | List AI models |
| `hu disk known` | Scan known space hogs |
| `hu disk list-known` | List known hog definitions |
| `hu disk inventory` | Full disk space analysis |
| `hu disk asdf` | Show ASDF version manager breakdown |

## Priority 6: Services (macOS)

### hu services
| Command | Description |
|---------|-------------|
| `hu services summary` | Overview of all services |
| `hu services brew` | Manage Homebrew services |
| `hu services login` | List login items and launch agents |
| `hu services launch` | Manage launch agents/daemons |
| `hu services memory` | List processes using most memory |

## Priority 7: Utilities

### hu utils (missing subcommands)
| Command | Description |
|---------|-------------|
| `hu utils plist <file>` | Read plist as colorized JSON |
| `hu utils sync-checkboxes` | Clean TODO.md/PLAN.md |

### hu bump
| Command | Description |
|---------|-------------|
| `hu bump run` | Bump package version |
| `hu bump detect` | Detect version format from git tags |

### hu docs
| Command | Description |
|---------|-------------|
| `hu docs list` | List documentation files |
| `hu docs archive` | Archive docs to global store |
| `hu docs check` | Check system document status |
| `hu docs scan` | Scan docs for sync |

### hu plans
| Command | Description |
|---------|-------------|
| `hu plans list` | List saved plans |
| `hu plans clear` | Delete all plans |

### hu plugin
| Command | Description |
|---------|-------------|
| `hu plugin create` | Create a Claude Code plugin |

## Priority 8: Code Analysis

### hu code
| Command | Description |
|---------|-------------|
| `hu code analyze` | Analyze code structure (imports, exports, definitions) |
| `hu code imports` | List imports in a file |
| `hu code defs` | List function/class definitions |
| `hu code exports` | List exports from a file |
| `hu code langs` | Detect languages in codebase |

## Currently Working

These commands are implemented and working in the Rust CLI:

```
hu context track/check/summary/clear
hu read --outline/--interface/--around/--diff
hu gh login/prs/runs/failures/ci
hu jira auth/tickets/sprint/search/show/update
hu utils fetch-html/grep/web-search/docs-index/docs-search/docs-section
hu dashboard show/refresh
hu eks list/exec/logs
hu slack messages/channels/send
hu pagerduty oncall/alerts/incidents
hu sentry issues/show
hu newrelic incidents/query
```
