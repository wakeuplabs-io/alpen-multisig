//! Registers the Tauri invoke handler set for production vs dev signing (P-003, P-040).

use desktop_app::infrastructure::dev_secrets;
use tauri::Wry;

pub fn attach_invoke_handlers(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    if dev_secrets::dev_mnemonic_signing_ipc_enabled() {
        attach_with_dev_signing(builder)
    } else {
        attach_production(builder)
    }
}

fn attach_production(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        super::asm_state::get_multisig_config,
        super::action_builder::build_admin_multisig_update_hex,
        super::action_builder::decode_action_hex,
        super::authentication::auth_start_challenge,
        super::authentication::auth_complete,
        super::authentication::auth_get_session,
        super::authentication::auth_logout,
        super::orchestrator_auth::orchestrator_auth_start,
        super::orchestrator_auth::orchestrator_auth_complete,
        super::orchestrator_auth::orchestrator_auth_get_session,
        super::orchestrator_auth::orchestrator_auth_logout,
        super::proposals::proposals_get_next_seq_no,
        super::proposals::proposals_create,
        super::proposals::proposals_list,
        super::proposals::proposals_get,
        super::proposals::proposals_approve,
        super::proposals::proposals_prepare_broadcast,
        super::proposals::proposals_broadcast,
        super::hw_wallet::get_trezor_info,
        super::hw_wallet::list_hw_addresses,
        super::hw_wallet::verify_address_on_device,
        super::hw_wallet::sign_with_trezor,
        super::hw_wallet::sign_challenge_with_trezor,
        super::hw_wallet::list_ledger_addresses,
        super::hw_wallet::get_ledger_info,
        super::hw_wallet::sign_with_ledger,
        super::hw_wallet::sign_challenge_with_ledger,
        super::signing::compute_sighash,
        super::signing::verify_threshold,
    ])
}

fn attach_with_dev_signing(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        super::asm_state::get_multisig_config,
        super::action_builder::build_admin_multisig_update_hex,
        super::action_builder::decode_action_hex,
        super::authentication::auth_start_challenge,
        super::authentication::auth_complete,
        super::authentication::auth_get_session,
        super::authentication::auth_logout,
        super::orchestrator_auth::orchestrator_auth_start,
        super::orchestrator_auth::orchestrator_auth_complete,
        super::orchestrator_auth::orchestrator_auth_get_session,
        super::orchestrator_auth::orchestrator_auth_logout,
        super::proposals::proposals_get_next_seq_no,
        super::proposals::proposals_create,
        super::proposals::proposals_list,
        super::proposals::proposals_get,
        super::proposals::proposals_approve,
        super::proposals::proposals_prepare_broadcast,
        super::proposals::proposals_broadcast,
        super::hw_wallet::get_trezor_info,
        super::hw_wallet::list_hw_addresses,
        super::hw_wallet::verify_address_on_device,
        super::hw_wallet::sign_with_trezor,
        super::hw_wallet::sign_challenge_with_trezor,
        super::hw_wallet::list_ledger_addresses,
        super::hw_wallet::get_ledger_info,
        super::hw_wallet::sign_with_ledger,
        super::hw_wallet::sign_challenge_with_ledger,
        super::signing::compute_sighash,
        super::signing::verify_threshold,
        super::signing::sign_action_sighash,
        super::signing::list_mnemonic_addresses,
        super::signing::sign_with_mnemonic_path,
    ])
}
