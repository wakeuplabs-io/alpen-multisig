pub(crate) mod commit_reveal_key;
pub mod wallet;

pub use wallet::{
    get_external_address, load_admin_wallet, load_watch_only_admin_wallet, AdminWalletError,
};
