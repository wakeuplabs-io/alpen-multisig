use std::any::Any;

use crate::application::psbt_signer::PsbtSigner;
use bdk_wallet::bitcoin::psbt::Psbt;
use bdk_wallet::bitcoin::Network;

/// Hardware wallet PSBT signer — re-opens device by fingerprint at sign time.
/// Allowed on any network (mainnet, testnet, regtest).
pub struct HwPsbtSigner {
    pub master_fingerprint: u32,
    pub device_type: HwDeviceType,
    pub account_xpub: String,
    pub network: Network,
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
}

impl HwPsbtSigner {
    pub fn new(
        master_fingerprint: u32,
        device_type: HwDeviceType,
        account_xpub: String,
        network: Network,
    ) -> Self {
        Self {
            master_fingerprint,
            device_type,
            account_xpub,
            network,
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
