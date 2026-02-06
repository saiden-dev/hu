Install hooks and commands to Claude Code.

```bash
hu install run                  # Install to ~/.claude (global)
hu install run --local          # Install to ./.claude (local)
hu install run --force          # Override modified files
hu install run --hooks-only     # Install only hooks
hu install run --commands-only  # Install only commands
hu install run hooks/hu/pre-read   # Install specific component
```
