Execute a command in a pod (interactive shell by default).

```bash
hu eks exec <pod>                    # Open shell
hu eks exec <pod> -- ls -la          # Run command
hu eks exec <pod> -n namespace       # Specify namespace
```
