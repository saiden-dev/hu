mod config;

pub use config::{load_credentials, save_credentials, GithubCredentials};

#[allow(unused_imports)]
pub use config::{config_dir, Credentials};
