use crate::libraries::tick_math;
use crate::states::pool::REWARD_NUM;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
pub const TICK_ARRAY_SIZE_USIZE: usize = 60;
pub const TICK_ARRAY_SIZE: i32 = 60;

#[derive(Debug, Clone, Copy, Default)]
pub struct LimitOrderMatchResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub amm_fee_amount: u64,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TickState {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    pub reward_growths_outside_x64: [u128; REWARD_NUM],
    pub order_phase: u64,
    pub orders_amount: u64,
    pub part_filled_orders_remaining: u64,
    pub unfilled_ratio_x64: u128,
    pub padding: [u32; 3],
}

impl TickState {
    pub const LEN: usize = 4 + 16 + 16 + 16 + 16 + 16 * REWARD_NUM + 4 * 8 + 16 + 4 * 1;

    pub fn check_is_out_of_boundary(tick: i32) -> bool {
        tick < tick_math::MIN_TICK || tick > tick_math::MAX_TICK
    }

    pub fn is_initialized(&self) -> bool {
        self.liquidity_gross != 0 || self.orders_amount != 0 || self.part_filled_orders_remaining != 0
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TickArrayState {
    pub pool_id: Pubkey,
    pub start_tick_index: i32,
    pub ticks: [TickState; TICK_ARRAY_SIZE_USIZE],
    pub initialized_tick_count: u8,
    pub recent_epoch: u64,
    pub padding: [u8; 107],
}

impl crate::util::AccountSchema for TickArrayState {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::TICK_ARRAY_STATE;
}

impl TickArrayState {
    pub const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::TICK_ARRAY_STATE;

    pub const LEN: usize = 8 + 32 + 4 + TickState::LEN * TICK_ARRAY_SIZE_USIZE + 1 + 115;

    pub fn tick_count(tick_spacing: u16) -> i32 {
        TICK_ARRAY_SIZE * i32::from(tick_spacing)
    }

    pub fn get_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = Self::tick_count(tick_spacing);
        let mut start = tick_index / ticks_in_array;
        if tick_index < 0 && tick_index % ticks_in_array != 0 {
            start -= 1;
        }
        start * ticks_in_array
    }

    pub fn check_is_valid_start_index(tick_index: i32, tick_spacing: u16) -> bool {
        if TickState::check_is_out_of_boundary(tick_index) {
            if tick_index > tick_math::MAX_TICK {
                return false;
            }
            let min_start_index = Self::get_array_start_index(tick_math::MIN_TICK, tick_spacing);
            return tick_index == min_start_index;
        }
        tick_index % Self::tick_count(tick_spacing) == 0
    }
}
