
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn disc(prefix: &str, name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(prefix.as_bytes());
    h.update(b":");
    h.update(name.as_bytes());
    let out = h.finalize();
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&out[..8]);
    arr
}

const ACCOUNTS: &[(&str, &str)] = &[
    ("AmmConfig", "AMM_CONFIG"),
    ("OperationState", "OPERATION_STATE"),
    ("ObservationState", "OBSERVATION_STATE"),
    ("PersonalPositionState", "PERSONAL_POSITION_STATE"),
    ("PoolState", "POOL_STATE"),
    ("ProtocolPositionState", "PROTOCOL_POSITION_STATE"),
    ("SupportMintAssociated", "SUPPORT_MINT_ASSOCIATED"),
    ("TickArrayState", "TICK_ARRAY_STATE"),
    ("TickArrayBitmapExtension", "TICK_ARRAY_BITMAP_EXTENSION"),
    ("DynamicFeeConfig", "DYNAMIC_FEE_CONFIG"),
    ("LimitOrderState", "LIMIT_ORDER_STATE"),
    ("LimitOrderNonceState", "LIMIT_ORDER_NONCE_STATE"),
];

const INSTRUCTIONS: &[(&str, &str)] = &[
    ("create_amm_config", "CREATE_AMM_CONFIG"),
    ("update_amm_config", "UPDATE_AMM_CONFIG"),
    ("create_support_mint_associated", "CREATE_SUPPORT_MINT_ASSOCIATED"),
    ("create_dynamic_fee_config", "CREATE_DYNAMIC_FEE_CONFIG"),
    ("update_dynamic_fee_config", "UPDATE_DYNAMIC_FEE_CONFIG"),
    ("create_pool", "CREATE_POOL"),
    ("create_customizable_pool", "CREATE_CUSTOMIZABLE_POOL"),
    ("update_pool_status", "UPDATE_POOL_STATUS"),
    ("create_operation_account", "CREATE_OPERATION_ACCOUNT"),
    ("update_operation_account", "UPDATE_OPERATION_ACCOUNT"),
    ("transfer_reward_owner", "TRANSFER_REWARD_OWNER"),
    ("initialize_reward", "INITIALIZE_REWARD"),
    ("collect_remaining_rewards", "COLLECT_REMAINING_REWARDS"),
    ("update_reward_infos", "UPDATE_REWARD_INFOS"),
    ("set_reward_params", "SET_REWARD_PARAMS"),
    ("collect_protocol_fee", "COLLECT_PROTOCOL_FEE"),
    ("collect_fund_fee", "COLLECT_FUND_FEE"),
    ("open_position", "OPEN_POSITION"),
    ("open_position_v2", "OPEN_POSITION_V2"),
    ("open_position_with_token22_nft", "OPEN_POSITION_WITH_TOKEN22_NFT"),
    ("close_position", "CLOSE_POSITION"),
    ("increase_liquidity", "INCREASE_LIQUIDITY"),
    ("increase_liquidity_v2", "INCREASE_LIQUIDITY_V2"),
    ("decrease_liquidity", "DECREASE_LIQUIDITY"),
    ("decrease_liquidity_v2", "DECREASE_LIQUIDITY_V2"),
    ("swap", "SWAP"),
    ("swap_v2", "SWAP_V2"),
    ("swap_router_base_in", "SWAP_ROUTER_BASE_IN"),
    ("close_protocol_position", "CLOSE_PROTOCOL_POSITION"),
    ("open_limit_order", "OPEN_LIMIT_ORDER"),
    ("increase_limit_order", "INCREASE_LIMIT_ORDER"),
    ("decrease_limit_order", "DECREASE_LIMIT_ORDER"),
    ("settle_limit_order", "SETTLE_LIMIT_ORDER"),
    ("close_limit_order", "CLOSE_LIMIT_ORDER"),
];

fn fmt_bytes(b: [u8; 8]) -> String {
    let parts: Vec<String> = b.iter().map(|x| format!("0x{:02x}", x)).collect();
    format!("[{}]", parts.join(", "))
}

fn main() {
    let mut out = String::new();
    out.push_str("pub mod account {\n");
    for (camel, screaming) in ACCOUNTS {
        let d = disc("account", camel);
        out.push_str(&format!(
            "    /// sha256(\"account:{}\")[..8]\n    pub const {}: [u8; 8] = {};\n",
            camel,
            screaming,
            fmt_bytes(d)
        ));
    }
    out.push_str("}\n\n");

    out.push_str("pub mod instruction {\n");
    for (snake, screaming) in INSTRUCTIONS {
        let d = disc("global", snake);
        out.push_str(&format!(
            "    /// sha256(\"global:{}\")[..8]\n    pub const {}: [u8; 8] = {};\n",
            snake,
            screaming,
            fmt_bytes(d)
        ));
    }
    out.push_str("}\n");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let path = PathBuf::from(out_dir).join("anchor_discriminators.rs");
    fs::write(&path, out).expect("failed to write anchor_discriminators.rs");

    println!("cargo:rerun-if-changed=build.rs");
}
