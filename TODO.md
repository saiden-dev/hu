# TODO

## Active: Jira ADF + Comments + Create (see PLAN.md)

### Phase 1 — Client split (refactor) ✅
- [x] 1.1 Extract `issues.rs` (get_issue, search_issues, update_issue, parse_user, parse_issues)
- [x] 1.2 Extract `transitions.rs` (get_transitions, transition_issue, parse_transitions)
- [x] 1.3 Slim `client/mod.rs` to trait + struct + new() + api_url()
- [x] 1.4 Verify `just check && just test` green (tarpaulin not exercised here)

### Cleanup ✅
- [x] cargo fmt + clippy fixes across crate (pre-existing tech debt)

### Phase 2 — ADF module + Markdown-by-default body ✅
- [x] 2.1 Add `pulldown-cmark = "0.12"` to Cargo.toml
- [x] 2.2 Create `src/jira/adf.rs` with `markdown_to_adf` + relocated `adf_to_plain_text` (22 inline tests instead of fs fixtures)
- [x] 2.3 Golden coverage via inline tests (deviation: skipped tests/fixtures/adf/ in favor of inline goldens — lower friction, same coverage)
- [x] 2.4 Wire `markdown_to_adf` into `update_issue` (replaces inline ADF)
- [x] 2.5 Add `--body-adf <PATH>` flag (mutex with `--body`)
- [ ] 2.6 Smoke test on Marketer instance — verify markdown headers, bold, lists render (manual; pending)
- [ ] 2.7 Release-notes blurb for v0.2.0 breaking change (deferred to wrap-up)

### Phase 3 — Read comments
- [ ] 3.1 Add `Comment` type in `types.rs`
- [ ] 3.2 Add `JiraApi::list_comments` trait method + 8 mock stubs
- [ ] 3.3 Implement `client/comments.rs`
- [ ] 3.4 Implement `src/jira/comments.rs` handler (table + json output)
- [ ] 3.5 Wire `Comments` subcommand in `cli.rs`
- [ ] 3.6 Tests: process_comments, format_comments, mock-driven coverage

### Phase 4 — Create issues
- [ ] 4.1 Add `IssueCreate` + `IssueType` types
- [ ] 4.2 Add `JiraApi::create_issue` and `JiraApi::get_create_meta` trait methods + 8 mock stubs
- [ ] 4.3 Implement `client/create.rs` with `OnceCell` createmeta cache
- [ ] 4.4 Implement `src/jira/create.rs` handler with fuzzy issue-type match
- [ ] 4.5 Read `default_project` from `~/.config/hu/jira.toml`
- [ ] 4.6 Wire `Create` subcommand in `cli.rs`
- [ ] 4.7 Tests: process_create, find_issue_type, mock-driven coverage
- [ ] 4.8 Smoke test: create a real ticket on Marketer, verify URL output

### Wrap-up
- [ ] Update README.md (new subcommands, `--body-adf`, breaking change note)
- [ ] Update `doc/` references
- [ ] Bump version to 0.2.0
- [ ] Update calibration in `workflow.eta_calibration` memory with actual times

---

## Deferred / Future

- [ ] Delete issues (`hu jira delete`)
- [ ] Add comment (`hu jira comment <KEY> --add "..."`)
- [ ] ADF tables support
- [ ] Granular OAuth scope migration

## Token-Saving CLI Features (older, unrelated track)

### Phase 4: Code Navigation (Optional/Future)
- [ ] Step 4.1: Add tree-sitter dependency
- [ ] Step 4.2: Implement symbol extraction
- [ ] Step 4.3: Implement structure and callers
- [ ] Step 4.4: Add CLI and tests
