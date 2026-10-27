use strata_asm_params::AsmParams;

/// The repo example must deserialize against the pinned `AsmParams` shape. Substitute the
/// blkid placeholder the same way an operator does before bootstrap, so a pin bump that adds
/// required fields fails here instead of in someone's first `asm-params.json` edit.
#[test]
fn asm_params_example_deserializes_after_blkid_placeholder_substituted() {
    let filled = include_str!("../../scripts/asm-params.example.json").replace(
        "REPLACE_WITH_64_HEX_CHARS_FROM_getblockhash_101",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let _: AsmParams =
        serde_json::from_str(&filled).expect("asm-params.example.json deserializes as AsmParams");
}

/// The staging template is what the Docker stack boots from, filled by `entrypoint-asm.sh`.
#[test]
fn staging_params_template_deserializes_after_blkid_placeholder_substituted() {
    let filled = include_str!("../../staging/asm-params.template.json").replace(
        "ANCHOR_BLKID_PLACEHOLDER",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let _: AsmParams =
        serde_json::from_str(&filled).expect("asm-params.template.json deserializes as AsmParams");
}
