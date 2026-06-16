use crate::states::pool::REWARD_NUM;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ProtocolPositionState {
    pub bump: u8,
    pub pool_id: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    pub token_fees_owed_0: u64,
    pub token_fees_owed_1: u64,
    pub reward_growth_inside: [u128; REWARD_NUM],
    pub recent_epoch: u64,
    pub padding: [u64; 7],
}

impl ProtocolPositionState {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::PROTOCOL_POSITION_STATE;

    pub const LEN: usize =
        8 + 1 + 32 + 4 + 4 + 16 + 16 + 16 + 8 + 8 + 16 * REWARD_NUM + 8 + 8 * 7;
}

impl crate::util::AccountSchema for ProtocolPositionState {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::PROTOCOL_POSITION_STATE;
}
