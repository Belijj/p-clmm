use crate::states::pool_fee::DynamicFeeInfo;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const POOL_SEED: &[u8] = b"pool";
pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";
pub const POOL_REWARD_VAULT_SEED: &[u8] = b"pool_reward_vault";
pub const POOL_TICK_ARRAY_BITMAP_SEED: &[u8] = b"pool_tick_array_bitmap_extension";
pub const REWARD_NUM: usize = 3;

pub enum PoolStatusBitIndex {
    OpenPositionOrIncreaseLiquidity = 0,
    DecreaseLiquidity = 1,
    CollectFee = 2,
    CollectReward = 3,
    Swap = 4,
    LimitOrder = 5,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RewardState {
    Uninitialized = 0,
    Initialized = 1,
    Opening = 2,
    Ended = 3,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct RewardInfo {
    pub reward_state: u8,
    pub open_time: u64,
    pub end_time: u64,
    pub last_update_time: u64,
    pub emissions_per_second_x64: u128,
    pub reward_total_emitted: u64,
    pub reward_claimed: u64,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub authority: Pubkey,
    pub reward_growth_global_x64: u128,
}

impl RewardInfo {
    pub const LEN: usize = 1 + 8 + 8 + 8 + 16 + 8 + 8 + 32 + 32 + 32 + 16;

    pub fn initialized(&self) -> bool {
        self.token_mint != Pubkey::default()
    }

    #[inline]
    pub fn get_reward_growths(infos: &[RewardInfo; REWARD_NUM]) -> [u128; REWARD_NUM] {
        let mut out = [0u128; REWARD_NUM];
        let mut i = 0;
        while i < REWARD_NUM {
            out[i] = infos[i].reward_growth_global_x64;
            i += 1;
        }
        out
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PoolState {
    pub bump: [u8; 1],
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub padding5: [u128; 4],
    pub status: u8,
    pub fee_on: u8,
    pub padding: [u8; 6],
    pub reward_infos: [RewardInfo; REWARD_NUM],
    pub tick_array_bitmap: [u64; 16],
    pub padding6: [u64; 4],
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub dynamic_fee_info: DynamicFeeInfo,
    pub padding1: [u64; 14],
    pub padding2: [u64; 32],
}

impl crate::util::AccountSchema for PoolState {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::POOL_STATE;
}

impl PoolState {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::POOL_STATE;

    pub const LEN: usize = 8
        + 1
        + 32 * 7
        + 1
        + 1
        + 2
        + 16
        + 16
        + 4
        + 2
        + 2
        + 16
        + 16
        + 8
        + 8
        + 16
        + 16
        + 16
        + 16
        + 8
        + RewardInfo::LEN * REWARD_NUM
        + 8 * 16
        + 8 * 8
        + DynamicFeeInfo::LEN
        + 8 * 14
        + 8 * 32;
}
