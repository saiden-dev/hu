use super::*;
use std::cell::RefCell;

/// Mock store for testing
struct MockStore {
    state: RefCell<ContextState>,
    session_id: String,
    deleted: RefCell<bool>,
}

impl MockStore {
    fn new() -> Self {
        Self {
            state: RefCell::new(ContextState::new("mock-session".to_string())),
            session_id: "mock-session".to_string(),
            deleted: RefCell::new(false),
        }
    }

    fn with_state(state: ContextState) -> Self {
        let session_id = state.session_id.clone();
        Self {
            state: RefCell::new(state),
            session_id,
            deleted: RefCell::new(false),
        }
    }
}

impl ContextStore for MockStore {
    fn load(&self) -> Result<ContextState> {
        Ok(self.state.borrow().clone())
    }

    fn save(&self, state: &ContextState) -> Result<()> {
        *self.state.borrow_mut() = state.clone();
        Ok(())
    }

    fn delete(&self) -> Result<()> {
        *self.deleted.borrow_mut() = true;
        Ok(())
    }
}

#[test]
fn format_age_seconds() {
    assert_eq!(format_age(0), "0s ago");
    assert_eq!(format_age(30), "30s ago");
    assert_eq!(format_age(59), "59s ago");
}

#[test]
fn format_age_minutes() {
    assert_eq!(format_age(60), "1m ago");
    assert_eq!(format_age(120), "2m ago");
    assert_eq!(format_age(3599), "59m ago");
}

#[test]
fn format_age_hours() {
    assert_eq!(format_age(3600), "1h ago");
    assert_eq!(format_age(7200), "2h ago");
    assert_eq!(format_age(86399), "23h ago");
}

#[test]
fn format_age_days() {
    assert_eq!(format_age(86400), "1d ago");
    assert_eq!(format_age(172800), "2d ago");
}

#[test]
fn format_bytes_b() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1023), "1023 B");
}

#[test]
fn format_bytes_kb() {
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(2048), "2.0 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
}

#[test]
fn format_bytes_mb() {
    assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
    assert_eq!(format_bytes(2 * 1024 * 1024), "2.0 MB");
}

#[test]
fn get_file_status_loaded() {
    let mut state = ContextState::new("s".to_string());
    state.track(ContextEntry::with_timestamp(
        PathBuf::from("/test.rs"),
        100,
        10,
        1000,
    ));

    let status = get_file_status(&state, &PathBuf::from("/test.rs"), 1060).unwrap();
    if let FileStatus::Loaded { entry, age_secs } = status {
        assert_eq!(entry.size, 100);
        assert_eq!(entry.line_count, 10);
        assert_eq!(age_secs, 60);
    } else {
        panic!("Expected Loaded");
    }
}

#[test]
fn get_file_status_not_loaded() {
    let state = ContextState::new("s".to_string());
    // Use Cargo.toml which we know exists
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let status = get_file_status(&state, &path, 1000).unwrap();
    assert!(matches!(status, FileStatus::NotLoaded { .. }));
}

#[test]
fn clear_with_store_deletes() {
    let store = MockStore::new();
    clear_with_store(&store).unwrap();
    assert!(*store.deleted.borrow());
}

#[test]
fn summary_with_store_empty() {
    let store = MockStore::new();
    // Just verify it doesn't panic
    summary_with_store(&store).unwrap();
}

#[test]
fn summary_with_store_with_entries() {
    let mut state = ContextState::new("test".to_string());
    state.track(ContextEntry::with_timestamp(
        PathBuf::from("/a.rs"),
        100,
        10,
        1000,
    ));
    state.track(ContextEntry::with_timestamp(
        PathBuf::from("/b.rs"),
        200,
        20,
        2000,
    ));
    let store = MockStore::with_state(state);
    summary_with_store(&store).unwrap();
}

#[test]
fn track_with_store_real_file() {
    let store = MockStore::new();
    let cargo_toml = env!("CARGO_MANIFEST_DIR").to_string() + "/Cargo.toml";
    track_with_store(&store, &[cargo_toml]).unwrap();

    let state = store.load().unwrap();
    assert_eq!(state.file_count(), 1);
}

#[test]
fn check_with_store_real_file() {
    let mut state = ContextState::new("test".to_string());
    let cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    state.track(ContextEntry::with_timestamp(
        cargo_path.clone(),
        100,
        10,
        current_timestamp() - 60,
    ));
    let store = MockStore::with_state(state);

    check_with_store(&store, &[cargo_path.to_string_lossy().to_string()]).unwrap();
}

#[test]
fn resolve_path_absolute() {
    let result = resolve_path("/tmp").unwrap();
    assert!(result.is_absolute());
}

#[test]
fn resolve_path_relative() {
    // Cargo.toml should exist in project root
    let result = resolve_path("Cargo.toml").unwrap();
    assert!(result.is_absolute());
    assert!(result.to_string_lossy().ends_with("Cargo.toml"));
}

#[test]
fn resolve_path_not_found() {
    let result = resolve_path("/nonexistent/path/to/file.xyz");
    assert!(result.is_err());
}

#[test]
fn get_file_info_real_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let (size, line_count) = get_file_info(&path).unwrap();
    assert!(size > 0);
    assert!(line_count > 0);
}

#[test]
fn get_file_info_missing_file() {
    let path = PathBuf::from("/nonexistent/file.txt");
    let result = get_file_info(&path);
    assert!(result.is_err());
}

#[test]
fn current_timestamp_returns_value() {
    let ts = current_timestamp();
    // Should be a reasonable Unix timestamp (after 2020)
    assert!(ts > 1577836800);
}

#[test]
fn mock_store_load_save() {
    let store = MockStore::new();
    let mut state = store.load().unwrap();
    state.track(ContextEntry::new(PathBuf::from("/test.rs"), 100, 10));
    store.save(&state).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.file_count(), 1);
}

#[test]
fn mock_store_session_id() {
    let store = MockStore::new();
    assert_eq!(store.session_id, "mock-session");
}

#[test]
fn print_file_status_loaded() {
    let entry = ContextEntry::with_timestamp(PathBuf::from("/test.rs"), 100, 10, 1000);
    let status = FileStatus::Loaded {
        entry,
        age_secs: 60,
    };
    // Just verify it doesn't panic - output goes to stdout
    print_file_status(&status);
}

#[test]
fn print_file_status_not_loaded() {
    let status = FileStatus::NotLoaded {
        path: PathBuf::from("/test.rs"),
        size: 100,
        line_count: 10,
    };
    // Just verify it doesn't panic - output goes to stdout
    print_file_status(&status);
}
