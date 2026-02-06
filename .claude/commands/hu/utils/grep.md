Smart grep with token-saving options.

```bash
hu utils grep "pattern" path/          # Search in path
hu utils grep "pattern" -g "*.rs"      # Filter by glob
hu utils grep "pattern" --refs         # File paths only
hu utils grep "pattern" -n 20          # Limit results
```
