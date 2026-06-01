use crate::application::psbt_signer::PsbtSigner;
use bdk_wallet::bitcoin::psbt::Psbt;
use bdk_wallet::bitcoin::Network;

/// Hardware wallet PSBT signer — re-opens device by fingerprint at sign time.
/// Allowed on any network (mainnet, testnet, regtest).
pub struct HwPsbtSigner {
    pub master_fingerprint: u32,
    pub device_type: HwDeviceType,
}

#[derive(Clone, Copy, Debug)]
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
    pub fn new(master_fingerprint: u32, device_type: HwDeviceType) -> Self {
        Self {
            master_fingerprint,
            device_type,
        }
    }
}

impl PsbtSigner for HwPsbtSigner {
    fn sign_psbt(&self, _psbt: &mut Psbt) -> Result<(), String> {
        // HW signing requires device interaction via spawn_blocking.
        // The actual signing is done by the caller (WalletService) which
        // has access to the Tauri event loop for device communication.
        // This method is called as part of the port interface to verify
        // the signer is allowed on the network.
        Ok(())
    }

    fn allowed_on(&self, _network: Network) -> bool {
        // Hardware wallets are allowed on all networks
        true
    }

    fn kind(&self) -> &str {
        self.device_type.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw_psbt_signer_allowed_on_all_networks() {
        let signer = HwPsbtSigner::new(0xDEADBEEF, HwDeviceType::Trezor);
        assert!(signer.allowed_on(Network::Bitcoin));
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));
        assert!(signer.allowed_on(Network::Signet));
    }
}
