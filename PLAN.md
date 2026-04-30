# PLAN: Jira ADF Rich Text + Comments + Create

**Branch:** `master` (in-place, single-repo session)
**Date:** 2026-04-30
**Scope:** Three features. **Delete is explicitly OUT of scope.**

1. Markdown → ADF for `--body` (default behavior; breaking change documented)
2. Read comments (`hu jira comments <KEY>`)
3. Create issues (`hu jira create ...`)

OAuth scopes already cover all three (`read:jira-work`, `write:jira-work`) — no auth changes.

---

## Architecture decisions (from architect + code-rust validation)

- **Refactor first.** `src/jira/client/mod.rs` is at 395/400 lines. Split into per-resource files before adding any new endpoint code.
- **`src/jira/adf.rs` as a sibling module** (not under `client/`). It is pure data — used by client, display, and any future viewer.
- **Single crate addition:** `pulldown-cmark = "0.12"` (MSRV 1.71, under our 1.80).
- **Trait expansion is fine.** Add `list_comments`, `create_issue`, `get_create_meta` to existing `JiraApi`. **8 mocks** in `tickets.rs:312`, `sprint.rs:214`, `show.rs:217`, `search.rs:228`, `service.rs:81`, `update/tests.rs:122` (and friends) need stub impls.
- **Createmeta endpoint:** use the new `/rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes` (the legacy `?projectKeys=` flavour is deprecated).
- **Createmeta caching:** in-process only, `OnceCell<HashMap<ProjectKey, Vec<IssueType>>>` on the client struct. No on-disk cache.
- **Markdown is the new `--body` default.** `--plain` is NOT added; we document the breaking change in release notes for v0.2.0. Add `--body-adf <file.json>` raw passthrough as the escape hatch for power users.
- **Adf functions are flat** (`fn markdown_to_adf(&str) -> Value`), not a builder. Easier to test with golden fixtures.

---

## Phase breakdown

### Phase 1 — Refactor `client/` into submodules

Pure restructure; full test coverage maintained. **No behavior change.**

```
src/jira/client/
  mod.rs          # JiraApi trait + JiraClient struct + new() + api_url()
  issues.rs       # get_issue, search_issues, update_issue, parse_user, parse_issues
  transitions.rs  # get_transitions, transition_issue, parse_transitions
  tests.rs        # existing
```

`extract_text_from_adf_node` → moved to `src/jira/adf.rs` as `pub(crate) fn adf_to_plain_text(&Value) -> String`.

Verify: `just check && just test && cargo tarpaulin` all green.

### Phase 2 — `src/jira/adf.rs` + Markdown-by-default body

- New file `src/jira/adf.rs` with:
  - `pub fn markdown_to_adf(md: &str) -> serde_json::Value` returning `{type:"doc", version:1, content:[...]}`
  - `pub fn adf_to_plain_text(node: &serde_json::Value) -> String` (relocated)
  - Internal helpers: `block_from_event`, `inline_from_event`, mark stacking
- `pulldown-cmark` parsing covers: headings 1-6, paragraphs, bold/italic/code/strike, links, bullet/ordered lists with nesting, code blocks (with language attr), blockquotes, hr, hard breaks. Tables deferred.
- Replace inline ADF in `update_issue` (currently `client/mod.rs:178-193`) with `markdown_to_adf(&update.description)`.
- Add `--body-adf <PATH>` flag on `hu jira update` for raw ADF JSON passthrough (mutually exclusive with `--body`).
- Golden fixtures: `tests/fixtures/adf/*.md` + `*.json` pairs covering each construct.

### Phase 3 — Read comments

- `types.rs`: add `Comment { id, author: User, body: String /* plain */, body_adf: Value, created: String, updated: String }`.
- `JiraApi::list_comments(&self, key: &str) -> Result<Vec<Comment>>`.
- `client/comments.rs`: implementation (`GET /issue/{key}/comment`), uses `adf_to_plain_text` to render `body`.
- `src/jira/comments.rs`: handler with table output (`comfy_table` UTF8_FULL_CONDENSED, per CLAUDE.md). Columns: when, author, body. Truncate body to N chars; full body on `--full`. JSON output via `-j/--json`.
- `cli.rs`: subcommand `Comments { key: String, #[arg(short, long)] full: bool, #[arg(short, long)] json: bool }`.
- Stub the new trait method in 8 mocks.

### Phase 4 — Create issues

- `types.rs`: add `IssueCreate { project_key, summary, issue_type, description: Option<Value>, assignee: Option<String> }` and `IssueType { id, name, description }`.
- `JiraApi::create_issue(&IssueCreate) -> Result<Issue>` and `JiraApi::get_create_meta(project_key: &str) -> Result<Vec<IssueType>>`.
- `client/create.rs`: POST `/issue` + GET `/issue/createmeta/{key}/issuetypes` with `OnceCell` cache.
- `src/jira/create.rs`: handler. Validates `--type` against createmeta; fuzzy match (case-insensitive, partial) like `update::find_transition`. Default project from config (`~/.config/hu/jira.toml` key `default_project`); fall back to `--project` flag.
- `cli.rs`: subcommand `Create { #[arg(short, long)] summary: String, #[arg(short, long, default_value = "Task")] r#type: String, #[arg(short, long)] project: Option<String>, #[arg(short, long)] body: Option<String>, #[arg(short, long)] assign: Option<String>, #[arg(short, long)] json: bool }`.
- Output: `✓ Created HU-1234: <summary>` + clickable URL `https://<site>.atlassian.net/browse/HU-1234`.
- Stub the new trait methods in 8 mocks.

---

## Estimates (cooperative Pilot + Titan velocity)

Calibration ratio: primitive-rich refactor 5-7x naive; new-module work 2-3x naive (ref: `workflow.eta_calibration` memory id 2256).

| Phase | Naive | Coop | Sessions | Notes |
|-------|-------|------|----------|-------|
| 1 — Client split | 4 h | ~45 min | 1 | Pure mechanical; tests are the safety net |
| 2 — ADF + Markdown body | 6 h | ~2 h | 1 | Real new code; golden fixtures dominate |
| 3 — Comments | 3 h | ~45 min | 0.5 | Mirrors existing list patterns |
| 4 — Create | 4 h | ~1.5 h | 0.5 | Createmeta + validation is the only nuance |
| **Total** | **17 h** | **~5 h** | **2 sessions** | |

Two sessions because phase 2 is a natural break (new module lands, 100% coverage proven) before pivoting to endpoints.

---

## Risk register

- **`--body` semantic change is breaking.** Bump to v0.2.0; release notes must call out: "any `--body` input containing `*`, `_`, `#`, backticks, or links now renders as Markdown." Mitigation: `--body-adf` escape hatch; users can fall back to plain via empty markdown (no markup chars triggers no-op pass-through).
- **ADF schema drift.** Pin `version: 1` constant. Atlassian rarely changes ADF v1, but watch for it.
- **`pulldown-cmark` event stream edge cases.** Specifically: empty paragraphs (collapse), soft-break-as-space vs hard-break-as-`hardBreak` node, links inside emphasis (mark stacking on text node), code fences with unknown language (pass through as `attrs.language`).
- **Createmeta deprecation.** Use new endpoint per Atlassian 2024 changelog. Old endpoint still works but warns.
- **Project permission for create.** User needs "Create Issues" project permission. Surface 403 with a project-level hint, not a generic auth error.
- **Mock surface.** 8 `MockJiraClient impl JiraApi` blocks — compiler will catch all of them, but plan a single sweep to add `unimplemented!()` stubs in lockstep with each trait expansion.

---

## Out of scope (deliberate)

- Delete issues — Pilot deferred; revisit later.
- Add comment (`hu jira comment <KEY> --add "..."`). Trivial after phase 3 lands but not on this branch's manifest.
- ADF tables (markdown `|` syntax) — defer to v0.3 unless needed.
- On-disk createmeta cache.
- Migration to granular OAuth scopes.

---

## Definition of done

- `just check` and `just test` pass on master.
- `cargo tarpaulin` reports 100% coverage on all new modules (per CLAUDE.md hard rule).
- README.md and `doc/` updated for new subcommands and `--body-adf` flag.
- Manual smoke test on Marketer Jira instance (atlassian.net): update with markdown body renders correctly; comments list shows real thread; create issue produces a valid key + URL.
- Release notes draft for v0.2.0 calling out the `--body` Markdown default.
