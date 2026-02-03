Improve test coverage one file at a time.

## Process

1. Run `cargo tarpaulin --out stdout` to get coverage report
2. Parse the per-file coverage from the output (lines like `src/foo.rs: X/Y`)
3. Find the file with the **highest coverage percentage** that is NOT yet 100%
4. Read that file and identify the uncovered lines (listed in "Uncovered Lines" section)
5. Write tests to cover those missing lines
6. Run tests to verify they pass
7. Run tarpaulin again to confirm the file is now at 100%
8. Stop and report:
   - Which file was fixed
   - What lines were missing
   - What tests were added
   - New coverage percentage

## Rules

- Only fix ONE file per invocation
- Prioritize files closest to 100% (easiest wins)
- If a line is genuinely untestable (e.g., unreachable error handling), explain why
- Follow the project's testing patterns (mock traits, separate logic from I/O)
- Do not skip files - if the highest coverage file is hard to test, still attempt it

## Example Output

```
Fixed: src/newrelic/display.rs
- Was: 125/126 (99.21%)
- Now: 126/126 (100%)
- Missing: Line 16 (DateTime::from_timestamp returns None for out-of-range)
- Added: test_format_time_out_of_range
```
