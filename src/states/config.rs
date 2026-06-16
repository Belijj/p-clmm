use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const AMM_CONFIG_SEED: &[u8] = b"amm_config";

pub const FEE_RATE_DENOMINATOR_VALUE: u32 = 1_000_000;

pub const MAX_TICK_SPACING: u16 = 1000;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AmmConfig {
    pub bump: u8,
    pub index: u16,
    pub owner: Pubkey,
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u16,
    pub fund_fee_rate: u32,
    pub padding_u32: u32,
    pub fund_owner: Pubkey,
    pub padding: [u64; 3],
}

impl crate::util::AccountSchema for AmmConfig {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::AMM_CONFIG;
}

impl AmmConfig {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::AMM_CONFIG;

    pub const LEN: usize = 8 + 1 + 2 + 32 + 4 + 4 + 2 + 4 + 4 + 32 + 8 * 3;
}
