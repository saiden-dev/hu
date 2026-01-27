# Quick Checklist & Metrics

## Metrics Targets

| Metric | Target |
|--------|--------|
| Max file size | <400 lines |
| Max function size | <50 lines |
| Max nesting depth | 3 levels |
| Max parameters | 5 |
| Magic numbers | 0 (use constants) |
| Duplicate code blocks | <3 patterns |

## Pre-Commit Checklist

### Structure
- [ ] Files under 400 lines
- [ ] Functions under 50 lines
- [ ] Nesting depth <= 3 levels

### Style (Ruby/Python)
- [ ] Predicates use `is_`, `has_`, `can_` prefixes
- [ ] Iterator chains over imperative loops
- [ ] Early returns to flatten logic
- [ ] Builder pattern for complex construction
- [ ] All types derive `Debug`

### Quality
- [ ] No magic numbers (use constants)
- [ ] No hardcoded URLs
- [ ] All errors propagated with context
- [ ] Common patterns extracted to helpers

### Testing
- [ ] **100% coverage** (`cargo tarpaulin`)
- [ ] Code is testable (logic separated from I/O)
- [ ] External deps use traits (mockable)
- [ ] Unit tests inline with `#[cfg(test)]` modules
- [ ] Integration tests in `tests/` for binary behavior
- [ ] CLI: help exits 0, outputs to stdout
- [ ] No dead code — use `unreachable!()` for impossible paths

### Tooling
- [ ] `clippy.toml` and `rustfmt.toml` configured
- [ ] CI runs fmt, clippy, and tests

## Commands

```bash
just check      # fmt + clippy
just test       # run all tests
just build      # build debug
just release    # build release
```
