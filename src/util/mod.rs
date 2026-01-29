mod config;

pub use config::{load_credentials, save_credentials, GithubCredentials, JiraCredentials};

#[allow(unused_imports)]
pub use config::{config_dir, Credentials};

// These are used in tests
#[allow(unused_imports)]
pub use config::{load_credentials_from, save_credentials_to};
