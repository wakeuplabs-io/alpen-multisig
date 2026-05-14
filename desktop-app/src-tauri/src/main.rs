#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::asm_state::get_multisig_config,
            commands::action_builder::build_admin_multisig_update_hex,
            commands::action_builder::decode_action_hex,
            commands::authentication::auth_start_challenge,
            commands::authentication::auth_complete,
            commands::authentication::auth_get_session,
            commands::authentication::auth_logout,
            commands::orchestrator_auth::orchestrator_auth_start,
            commands::orchestrator_auth::orchestrator_auth_complete,
            commands::orchestrator_auth::orchestrator_auth_get_session,
            commands::orchestrator_auth::orchestrator_auth_logout,
            commands::proposals::proposals_get_next_seq_no,
            commands::proposals::proposals_create,
            commands::proposals::proposals_list,
            commands::proposals::proposals_get,
            commands::proposals::proposals_approve,
            commands::proposals::proposals_prepare_broadcast,
            commands::proposals::proposals_broadcast,
            commands::hw_wallet::get_trezor_info,
            commands::hw_wallet::list_hw_addresses,
            commands::hw_wallet::verify_address_on_device,
            commands::hw_wallet::sign_with_trezor,
            commands::hw_wallet::sign_challenge_with_trezor,
            commands::signing::compute_sighash,
            commands::signing::verify_threshold,
            commands::signing::sign_action_sighash,
            commands::signing::list_mnemonic_addresses,
            commands::signing::sign_with_mnemonic_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
