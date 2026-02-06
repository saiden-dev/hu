Smart file reading with outline, interface, around, and diff modes.

```bash
hu read src/main.rs                        # Full file
hu read src/main.rs -o                     # Outline (functions, structs, classes)
hu read src/main.rs -i                     # Public interface only
hu read src/main.rs -a 42                  # Lines around line 42
hu read src/main.rs -a 42 -n 20            # 20 context lines around line 42
hu read src/main.rs -d                     # Git diff (vs HEAD)
hu read src/main.rs -d --commit abc123     # Diff against specific commit
```

| Flag | Description |
|------|-------------|
| `-o, --outline` | Show file outline (functions, structs, classes) |
| `-i, --interface` | Public interface only (pub items in Rust, exports in JS) |
| `-a, --around` | Show lines around a specific line number |
| `-n, --context` | Context lines for `--around` (default: 10) |
| `-d, --diff` | Show git diff |
| `--commit` | Commit to diff against (default: HEAD) |
