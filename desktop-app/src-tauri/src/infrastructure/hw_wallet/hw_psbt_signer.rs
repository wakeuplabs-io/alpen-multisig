use std::any::Any;
use std::sync::Arc;

use crate::application::psbt_signer::PsbtSigner;
use crate::infrastructure::hw_wallet::{ledger, trezor};
use bdk_wallet::bitcoin::psbt::Psbt;
use bdk_wallet::bitcoin::Network;

/// On-device PSBT signing operation, parameterized by the wallet's account xpub,
/// master fingerprint, and network. Wired to the concrete device adapter at
/// construction (by [`HwDeviceType`]); injectable in tests via [`HwPsbtSigner::with_device_sign`].
///
/// Runs on a blocking thread (the adapters open their own device session), so it is
/// `Send + Sync` and takes `&mut Psbt`.
pub type DeviceSignFn =
    Arc<dyn Fn(&mut Psbt, &str, u32, Network) -> Result<(), String> + Send + Sync>;

/// Hardware wallet PSBT signer — re-opens device by fingerprint at sign time.
/// Allowed on any network (mainnet, testnet, regtest).
pub struct HwPsbtSigner {
    pub master_fingerprint: u32,
    pub device_type: HwDeviceType,
    pub account_xpub: String,
    pub network: Network,
    device_sign: DeviceSignFn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HwDeviceType {
    Trezor,
    Ledger,
}

impl HwDeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HwDeviceType::Trezor => "trezor",
            HwDeviceType::Ledger => "ledger",
        }
    }

    /// The concrete adapter PSBT-signing function for this device.
    fn device_sign_fn(self) -> DeviceSignFn {
        match self {
            HwDeviceType::Trezor => Arc::new(trezor::sign_admin_wallet_psbt),
            HwDeviceType::Ledger => Arc::new(ledger::sign_admin_wallet_psbt),
        }
    }
}

impl HwPsbtSigner {
    pub fn new(
        master_fingerprint: u32,
        device_type: HwDeviceType,
        account_xpub: String,
        network: Network,
    ) -> Self {
        let device_sign = device_type.device_sign_fn();
        Self {
            master_fingerprint,
            device_type,
            account_xpub,
            network,
            device_sign,
        }
    }

    /// The wired on-device signing operation (clone of the `Arc`), driven by
    /// [`WalletService::sign_and_finalize_psbt`] on a blocking thread.
    pub(crate) fn device_sign(&self) -> DeviceSignFn {
        Arc::clone(&self.device_sign)
    }

    /// Test-only constructor injecting a stub signing operation, so the dispatch
    /// seam and the "no broadcast on sign failure" guard can be exercised without
    /// a physical/emulated device.
    #[cfg(test)]
    pub(crate) fn with_device_sign(
        master_fingerprint: u32,
        device_type: HwDeviceType,
        account_xpub: String,
        network: Network,
        device_sign: DeviceSignFn,
    ) -> Self {
        Self {
            master_fingerprint,
            device_type,
            account_xpub,
            network,
            device_sign,
        }
    }
}

impl PsbtSigner for HwPsbtSigner {
    fn sign_psbt(&self, _wallet: &mut bdk_wallet::Wallet, _psbt: &mut Psbt) -> Result<(), String> {
        Err(
            "hardware PSBT signing must run on a blocking thread (see WalletService::build_and_sign_tx)"
                .to_string(),
        )
    }

    fn allowed_on(&self, _network: Network) -> bool {
        // Hardware wallets are allowed on all networks
        true
    }

    fn kind(&self) -> &str {
        self.device_type.as_str()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw_psbt_signer_allowed_on_all_networks() {
        let signer = HwPsbtSigner::new(
            0xDEADBEEF,
            HwDeviceType::Trezor,
            "tpubTEST".to_string(),
            Network::Regtest,
        );
        assert!(signer.allowed_on(Network::Bitcoin));
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));
        assert!(signer.allowed_on(Network::Signet));
    }
}
