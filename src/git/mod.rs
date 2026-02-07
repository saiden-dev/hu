mod service;
mod types;

pub use service::sync;
pub use types::{SyncOptions, SyncResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_are_accessible() {
        let _ = std::any::type_name::<SyncOptions>();
        let _ = std::any::type_name::<SyncResult>();
    }
}
