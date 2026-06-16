use crate::states::pool::REWARD_NUM;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const POSITION_SEED: &[u8] = b"position";

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct PositionRewardInfo {
    pub growth_inside_last_x64: u128,
    pub reward_amount_owed: u64,
}

impl PositionRewardInfo {
    pub const LEN: usize = 16 + 8;
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PersonalPositionState {
    pub bump: [u8; 1],
    pub nft_mint: Pubkey,
    pub pool_id: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    pub token_fees_owed_0: u64,
    pub token_fees_owed_1: u64,
    pub reward_infos: [PositionRewardInfo; REWARD_NUM],
    pub recent_epoch: u64,
    pub padding: [u64; 7],
}

impl PersonalPositionState {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::PERSONAL_POSITION_STATE;

    pub const LEN: usize = 8
        + 1
        + 32
        + 32
        + 4
        + 4
        + 16
        + 16
        + 16
        + 8
        + 8
        + PositionRewardInfo::LEN * REWARD_NUM
        + 8
        + 8 * 7;
}

impl crate::util::AccountSchema for PersonalPositionState {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::PERSONAL_POSITION_STATE;
}
