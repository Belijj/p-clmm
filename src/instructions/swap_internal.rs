
use crate::error::ErrorCode;
use crate::instructions::swap::TickArrayPath;
use crate::libraries::{
    fixed_point_64,
    full_math::MulDiv,
    liquidity_math, swap_math, tick_math,
    U128,
};
use crate::states::{
    AmmConfig, CollectFeeOn, ObservationState, PoolState, RewardInfo, TickArrayState, TickState,
    FEE_RATE_DENOMINATOR_VALUE, OBSERVATION_NUM, OBSERVATION_UPDATE_DURATION_DEFAULT, REWARD_NUM,
};
use crate::{require, Result};

#[derive(Debug, Clone, Copy)]
pub struct SwapInternalResult {
    pub amount_0: u64,
    pub amount_1: u64,
}

#[derive(Clone, Copy)]
struct SwapState {
    sqrt_price_x64: u128,
    liquidity: u128,
    fee_growth_global_input_x64: u128,
    protocol_fee_input: u64,
    fund_fee_input: u64,
    amount_specified_remaining: u64,
    amount_calculated: u64,
    tick_current: i32,
}

pub fn swap_internal<'a>(
    pool_state: &mut PoolState,
    tick_arrays: &mut TickArrayPath<'a>,
    amm_config: &AmmConfig,
    observation_state: &mut ObservationState,
    amount_specified: u64,
    sqrt_price_limit_x64: u128,
    zero_for_one: bool,
    is_base_input: bool,
    block_timestamp: u64,
) -> Result<SwapInternalResult> {
    require!(amount_specified != 0, ErrorCode::InvaildSwapAmountSpecified);

    update_pool_rewards(pool_state, block_timestamp)?;

    let tick_spacing: u16 = pool_state.tick_spacing;
    let total_fee_rate: u32 = amm_config.trade_fee_rate;
    let protocol_fee_rate: u32 = amm_config.protocol_fee_rate;
    let fund_fee_rate: u32 = amm_config.fund_fee_rate;
    let pool_fee_on = CollectFeeOn::from_u8(pool_state.fee_on);
    let is_fee_on_input = match pool_fee_on {
        CollectFeeOn::FromInput => true,
        CollectFeeOn::Token0Only => zero_for_one,
        CollectFeeOn::Token1Only => !zero_for_one,
    };

    let fee_growth_global_x64_at_start = if zero_for_one {
        pool_state.fee_growth_global_0_x64
    } else {
        pool_state.fee_growth_global_1_x64
    };

    let sqrt_price_x64_at_start: u128 = pool_state.sqrt_price_x64;
    require!(
        if zero_for_one {
            sqrt_price_limit_x64 < sqrt_price_x64_at_start
                && sqrt_price_limit_x64 > tick_math::MIN_SQRT_PRICE_X64
        } else {
            sqrt_price_limit_x64 > sqrt_price_x64_at_start
                && sqrt_price_limit_x64 < tick_math::MAX_SQRT_PRICE_X64
        },
        ErrorCode::SqrtPriceLimitOverflow
    );

    let reward_growths_at_start: [u128; REWARD_NUM] =
        RewardInfo::get_reward_growths(&pool_state.reward_infos);

    let mut state = SwapState {
        sqrt_price_x64: sqrt_price_x64_at_start,
        liquidity: pool_state.liquidity,
        fee_growth_global_input_x64: fee_growth_global_x64_at_start,
        protocol_fee_input: 0,
        fund_fee_input: 0,
        amount_specified_remaining: amount_specified,
        amount_calculated: 0,
        tick_current: pool_state.tick_current,
    };

    let mut current_array: &mut TickArrayState = tick_arrays
        .pop_front()
        .ok_or(ErrorCode::NotEnoughTickArrayAccount)?;
    while !array_contains_tick(current_array, state.tick_current, tick_spacing) {
        current_array = tick_arrays
            .pop_front()
            .ok_or(ErrorCode::NotEnoughTickArrayAccount)?;
    }

    while state.amount_specified_remaining != 0
        && state.sqrt_price_x64 != sqrt_price_limit_x64
    {
        let next_tick_index = match find_next_initialized_tick(
            current_array,
            state.tick_current,
            tick_spacing,
            zero_for_one,
        ) {
            Some(t) => t,
            None => {
                current_array = tick_arrays
                    .pop_front()
                    .ok_or(ErrorCode::LiquidityInsufficient)?;
                first_or_last_initialized_tick(current_array, tick_spacing, zero_for_one)
                    .ok_or(ErrorCode::LiquidityInsufficient)?
            }
        };

        let next_tick_sqrt_price = tick_math::get_sqrt_price_at_tick(next_tick_index)?;
        let step_target_price = if zero_for_one {
            core::cmp::max(next_tick_sqrt_price, sqrt_price_limit_x64)
        } else {
            core::cmp::min(next_tick_sqrt_price, sqrt_price_limit_x64)
        };

        let step = swap_math::compute_swap(
            state.sqrt_price_x64,
            step_target_price,
            state.liquidity,
            state.amount_specified_remaining,
            total_fee_rate,
            is_base_input,
            zero_for_one,
            is_fee_on_input,
        )?;

        if is_base_input {
            let consumed = if is_fee_on_input {
                step.amount_in
                    .checked_add(step.fee_amount)
                    .ok_or(ErrorCode::CalculateOverflow)?
            } else {
                step.amount_in
            };
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(consumed)
                .ok_or(ErrorCode::CalculateOverflow)?;
            state.amount_calculated = state
                .amount_calculated
                .checked_add(step.amount_out)
                .ok_or(ErrorCode::CalculateOverflow)?;
        } else {
            let received = if is_fee_on_input {
                step.amount_out
            } else {
                step.amount_out
                    .checked_add(step.fee_amount)
                    .ok_or(ErrorCode::CalculateOverflow)?
            };
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(received)
                .ok_or(ErrorCode::CalculateOverflow)?;
            state.amount_calculated = state
                .amount_calculated
                .checked_add(step.amount_in)
                .ok_or(ErrorCode::CalculateOverflow)?;
        }

        if step.fee_amount > 0 && is_fee_on_input {
            let protocol_cut = (step.fee_amount as u128)
                .checked_mul(protocol_fee_rate as u128)
                .ok_or(ErrorCode::CalculateOverflow)?
                / FEE_RATE_DENOMINATOR_VALUE as u128;
            let fund_cut = (step.fee_amount as u128)
                .checked_mul(fund_fee_rate as u128)
                .ok_or(ErrorCode::CalculateOverflow)?
                / FEE_RATE_DENOMINATOR_VALUE as u128;
            let lp_cut = (step.fee_amount as u128)
                .checked_sub(protocol_cut)
                .and_then(|v| v.checked_sub(fund_cut))
                .ok_or(ErrorCode::CalculateOverflow)?;

            state.protocol_fee_input = state
                .protocol_fee_input
                .checked_add(protocol_cut as u64)
                .ok_or(ErrorCode::CalculateOverflow)?;
            state.fund_fee_input = state
                .fund_fee_input
                .checked_add(fund_cut as u64)
                .ok_or(ErrorCode::CalculateOverflow)?;

            if state.liquidity > 0 {
                let scaled = U128::from(lp_cut as u64)
                    .mul_div_floor(U128::from(fixed_point_64::Q64), U128::from(state.liquidity))
                    .ok_or(ErrorCode::CalculateOverflow)?
                    .as_u128();
                state.fee_growth_global_input_x64 = state
                    .fee_growth_global_input_x64
                    .wrapping_add(scaled);
            }
        }

        state.sqrt_price_x64 = step.sqrt_price_next_x64;

        if state.sqrt_price_x64 == next_tick_sqrt_price
            && state.sqrt_price_x64 == step_target_price
        {
            let fee_growth_global_input_for_cross = state.fee_growth_global_input_x64;
            cross_tick(
                current_array,
                next_tick_index,
                tick_spacing,
                &mut state,
                fee_growth_global_input_for_cross,
                zero_for_one,
                fee_growth_global_x64_at_start,
                &reward_growths_at_start,
            )?;
            state.tick_current = if zero_for_one {
                next_tick_index - 1
            } else {
                next_tick_index
            };
        } else if state.sqrt_price_x64 != sqrt_price_x64_at_start {
            state.tick_current = tick_math::get_tick_at_sqrt_price(state.sqrt_price_x64)?;
        }
    }

    pool_state.sqrt_price_x64 = state.sqrt_price_x64;
    pool_state.tick_current = state.tick_current;
    pool_state.liquidity = state.liquidity;
    if zero_for_one {
        pool_state.fee_growth_global_0_x64 = state.fee_growth_global_input_x64;
        pool_state.protocol_fees_token_0 = pool_state
            .protocol_fees_token_0
            .checked_add(state.protocol_fee_input)
            .ok_or(ErrorCode::CalculateOverflow)?;
        pool_state.fund_fees_token_0 = pool_state
            .fund_fees_token_0
            .checked_add(state.fund_fee_input)
            .ok_or(ErrorCode::CalculateOverflow)?;
    } else {
        pool_state.fee_growth_global_1_x64 = state.fee_growth_global_input_x64;
        pool_state.protocol_fees_token_1 = pool_state
            .protocol_fees_token_1
            .checked_add(state.protocol_fee_input)
            .ok_or(ErrorCode::CalculateOverflow)?;
        pool_state.fund_fees_token_1 = pool_state
            .fund_fees_token_1
            .checked_add(state.fund_fee_input)
            .ok_or(ErrorCode::CalculateOverflow)?;
    }

    update_observation(observation_state, block_timestamp as u32, state.tick_current);

    let (amount_0, amount_1) = if is_base_input {
        let input_consumed = amount_specified
            .checked_sub(state.amount_specified_remaining)
            .ok_or(ErrorCode::CalculateOverflow)?;
        if zero_for_one {
            (input_consumed, state.amount_calculated)
        } else {
            (state.amount_calculated, input_consumed)
        }
    } else {
        let output_consumed = amount_specified
            .checked_sub(state.amount_specified_remaining)
            .ok_or(ErrorCode::CalculateOverflow)?;
        if zero_for_one {
            (state.amount_calculated, output_consumed)
        } else {
            (output_consumed, state.amount_calculated)
        }
    };

    Ok(SwapInternalResult { amount_0, amount_1 })
}

fn update_pool_rewards(pool_state: &mut PoolState, block_timestamp: u64) -> Result<()> {
    let any_active = pool_state
        .reward_infos
        .iter()
        .any(|r| r.initialized());
    if !any_active {
        return Ok(());
    }

    let liquidity: u128 = pool_state.liquidity;
    for i in 0..REWARD_NUM {
        let info = &pool_state.reward_infos[i];
        if !info.initialized() {
            continue;
        }
        if block_timestamp <= info.open_time {
            continue;
        }
        let end_time = info.end_time;
        let last_update_time = info.last_update_time;
        let emissions_per_second_x64 = info.emissions_per_second_x64;
        let total_emitted_so_far = info.reward_total_emitted;
        let growth_so_far = info.reward_growth_global_x64;

        let latest_update_timestamp = block_timestamp.min(end_time);
        let time_delta = latest_update_timestamp.saturating_sub(last_update_time);
        if time_delta == 0 {
            continue;
        }

        if liquidity > 0 {
            let reward_delta = U128::from(time_delta)
                .mul_div_ceil(
                    U128::from(emissions_per_second_x64),
                    U128::from(fixed_point_64::Q64),
                )
                .unwrap_or(U128::zero())
                .as_u64();

            let mut growth_delta_x64 = crate::libraries::big_num::U256::from(time_delta)
                .mul_div_floor(
                    crate::libraries::big_num::U256::from(emissions_per_second_x64),
                    crate::libraries::big_num::U256::from(liquidity),
                )
                .unwrap_or(crate::libraries::big_num::U256::zero());

            let (new_total, capped) =
                match total_emitted_so_far.checked_add(reward_delta) {
                    Some(v) => (v, false),
                    None => (u64::MAX, true),
                };

            if capped {
                let remainder = u64::MAX.saturating_sub(total_emitted_so_far);
                growth_delta_x64 = crate::libraries::big_num::U256::from(remainder)
                    .mul_div_floor(
                        crate::libraries::big_num::U256::from(fixed_point_64::Q64),
                        crate::libraries::big_num::U256::from(liquidity),
                    )
                    .unwrap_or(crate::libraries::big_num::U256::zero());
            }

            let new_growth = growth_so_far.wrapping_add(growth_delta_x64.as_u128());

            let info_mut = &mut pool_state.reward_infos[i];
            info_mut.reward_total_emitted = new_total;
            info_mut.reward_growth_global_x64 = new_growth;
            info_mut.last_update_time = latest_update_timestamp;
        } else {
            pool_state.reward_infos[i].last_update_time = latest_update_timestamp;
        }
    }
    Ok(())
}

fn array_contains_tick(array: &TickArrayState, tick_index: i32, tick_spacing: u16) -> bool {
    let start: i32 = array.start_tick_index;
    let span: i32 = TickArrayState::tick_count(tick_spacing);
    tick_index >= start && tick_index < start + span
}

fn find_next_initialized_tick(
    array: &TickArrayState,
    current_tick: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Option<i32> {
    let start = array.start_tick_index;
    let ts = i32::from(tick_spacing);
    if zero_for_one {
        let mut idx = ((current_tick - start) / ts) as isize;
        if idx >= array.ticks.len() as isize {
            idx = array.ticks.len() as isize - 1;
        }
        while idx >= 0 {
            let t = &array.ticks[idx as usize];
            let tick_index = start + (idx as i32) * ts;
            if t.is_initialized() && tick_index < current_tick {
                return Some(tick_index);
            }
            idx -= 1;
        }
        None
    } else {
        let mut idx = ((current_tick - start) / ts) as isize + 1;
        if idx < 0 {
            idx = 0;
        }
        while (idx as usize) < array.ticks.len() {
            let t = &array.ticks[idx as usize];
            let tick_index = start + (idx as i32) * ts;
            if t.is_initialized() && tick_index > current_tick {
                return Some(tick_index);
            }
            idx += 1;
        }
        None
    }
}

fn first_or_last_initialized_tick(
    array: &TickArrayState,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Option<i32> {
    let start = array.start_tick_index;
    let ts = i32::from(tick_spacing);
    let n = array.ticks.len();
    if zero_for_one {
        for i in (0..n).rev() {
            if array.ticks[i].is_initialized() {
                return Some(start + (i as i32) * ts);
            }
        }
    } else {
        for i in 0..n {
            if array.ticks[i].is_initialized() {
                return Some(start + (i as i32) * ts);
            }
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn cross_tick(
    array: &mut TickArrayState,
    tick_index: i32,
    tick_spacing: u16,
    state: &mut SwapState,
    fee_growth_global_input_x64: u128,
    zero_for_one: bool,
    fee_growth_global_other_at_start_x64: u128,
    reward_growths_at_start: &[u128; REWARD_NUM],
) -> Result<()> {
    let ts = i32::from(tick_spacing);
    let offset = ((tick_index - array.start_tick_index) / ts) as usize;
    if offset >= array.ticks.len() {
        return Err(ErrorCode::InvalidTickArray.into());
    }

    let tick: &mut TickState = &mut array.ticks[offset];

    let (new_outside_0, new_outside_1) = if zero_for_one {
        (
            fee_growth_global_input_x64.wrapping_sub(tick.fee_growth_outside_0_x64),
            fee_growth_global_other_at_start_x64.wrapping_sub(tick.fee_growth_outside_1_x64),
        )
    } else {
        (
            fee_growth_global_other_at_start_x64.wrapping_sub(tick.fee_growth_outside_0_x64),
            fee_growth_global_input_x64.wrapping_sub(tick.fee_growth_outside_1_x64),
        )
    };
    tick.fee_growth_outside_0_x64 = new_outside_0;
    tick.fee_growth_outside_1_x64 = new_outside_1;

    for i in 0..REWARD_NUM {
        tick.reward_growths_outside_x64[i] =
            reward_growths_at_start[i].wrapping_sub(tick.reward_growths_outside_x64[i]);
    }

    let liquidity_net = if zero_for_one {
        -tick.liquidity_net
    } else {
        tick.liquidity_net
    };
    state.liquidity = liquidity_math::add_delta(state.liquidity, liquidity_net)?;

    Ok(())
}

fn update_observation(state: &mut ObservationState, block_timestamp: u32, tick: i32) {
    let observation_index = state.observation_index as usize;
    if state.initialized == 0 {
        state.initialized = 1;
        state.observations[observation_index].block_timestamp = block_timestamp;
        state.observations[observation_index].tick_cumulative = 0;
        return;
    }
    let last = state.observations[observation_index];
    let last_ts = last.block_timestamp;
    let delta_time = block_timestamp.saturating_sub(last_ts);
    if delta_time < OBSERVATION_UPDATE_DURATION_DEFAULT {
        return;
    }
    let delta_cum = i64::from(tick).saturating_mul(i64::from(delta_time));
    let next_idx = if observation_index == OBSERVATION_NUM - 1 {
        0
    } else {
        observation_index + 1
    };
    state.observations[next_idx].block_timestamp = block_timestamp;
    let last_cum = last.tick_cumulative;
    state.observations[next_idx].tick_cumulative = last_cum.wrapping_add(delta_cum);
    state.observation_index = next_idx as u16;
}
