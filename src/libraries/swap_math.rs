use super::full_math::MulDiv;
use super::liquidity_math;
use super::sqrt_price_math;
use crate::error::ErrorCode;
use crate::states::config::FEE_RATE_DENOMINATOR_VALUE;
use crate::{Result, require_gte};

#[derive(Default, Debug)]
pub struct SwapComputationResult {
    pub sqrt_price_next_x64: u128,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
}

impl SwapComputationResult {
    pub fn new(sqrt_price_next_x64: u128) -> Self {
        Self {
            sqrt_price_next_x64,
            amount_in: 0,
            amount_out: 0,
            fee_amount: 0,
        }
    }
}

pub fn compute_swap(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u32,
    is_base_input: bool,
    zero_for_one: bool,
    is_fee_on_input: bool,
) -> Result<SwapComputationResult> {
    let mut result = SwapComputationResult::default();
    if is_base_input {
        let amount_for_price_calc = if is_fee_on_input {
            amount_remaining
                .mul_div_floor(
                    (FEE_RATE_DENOMINATOR_VALUE - fee_rate).into(),
                    u64::from(FEE_RATE_DENOMINATOR_VALUE),
                )
                .ok_or(ErrorCode::CalculateOverflow)?
        } else {
            amount_remaining
        };

        let amount_in = calculate_amount_in_range(
            sqrt_price_current_x64,
            sqrt_price_target_x64,
            liquidity,
            zero_for_one,
            is_base_input,
        )?;
        if let Some(v) = amount_in {
            result.amount_in = v;
        }

        result.sqrt_price_next_x64 =
            if amount_in.is_some() && amount_for_price_calc >= result.amount_in {
                sqrt_price_target_x64
            } else {
                sqrt_price_math::get_next_sqrt_price_from_input(
                    sqrt_price_current_x64,
                    liquidity,
                    amount_for_price_calc,
                    zero_for_one,
                )?
            };
    } else {
        let amount_for_price_calc = if is_fee_on_input {
            amount_remaining
        } else {
            amount_remaining
                .mul_div_ceil(
                    u64::from(FEE_RATE_DENOMINATOR_VALUE).into(),
                    (FEE_RATE_DENOMINATOR_VALUE - fee_rate).into(),
                )
                .ok_or(ErrorCode::CalculateOverflow)?
        };

        let amount_out = calculate_amount_in_range(
            sqrt_price_current_x64,
            sqrt_price_target_x64,
            liquidity,
            zero_for_one,
            is_base_input,
        )?;
        if let Some(v) = amount_out {
            result.amount_out = v;
        }
        result.sqrt_price_next_x64 =
            if amount_out.is_some() && amount_for_price_calc >= result.amount_out {
                sqrt_price_target_x64
            } else {
                sqrt_price_math::get_next_sqrt_price_from_output(
                    sqrt_price_current_x64,
                    liquidity,
                    amount_for_price_calc,
                    zero_for_one,
                )?
            }
    }

    if zero_for_one {
        require_gte!(result.sqrt_price_next_x64, sqrt_price_target_x64);
    } else {
        require_gte!(sqrt_price_target_x64, result.sqrt_price_next_x64);
    }

    let max = sqrt_price_target_x64 == result.sqrt_price_next_x64;
    if zero_for_one {
        if !(max && is_base_input) {
            result.amount_in = liquidity_math::get_delta_amount_0_unsigned(
                result.sqrt_price_next_x64,
                sqrt_price_current_x64,
                liquidity,
                true,
            )?
        };
        if !(max && !is_base_input) {
            result.amount_out = liquidity_math::get_delta_amount_1_unsigned(
                result.sqrt_price_next_x64,
                sqrt_price_current_x64,
                liquidity,
                false,
            )?;
        };
    } else {
        if !(max && is_base_input) {
            result.amount_in = liquidity_math::get_delta_amount_1_unsigned(
                sqrt_price_current_x64,
                result.sqrt_price_next_x64,
                liquidity,
                true,
            )?
        };
        if !(max && !is_base_input) {
            result.amount_out = liquidity_math::get_delta_amount_0_unsigned(
                sqrt_price_current_x64,
                result.sqrt_price_next_x64,
                liquidity,
                false,
            )?
        };
    }

    if is_base_input {
        if is_fee_on_input {
            if result.sqrt_price_next_x64 != sqrt_price_target_x64 {
                result.fee_amount = amount_remaining
                    .checked_sub(result.amount_in)
                    .ok_or(ErrorCode::CalculateOverflow)?;
            } else {
                result.fee_amount = result
                    .amount_in
                    .mul_div_ceil(
                        fee_rate.into(),
                        (FEE_RATE_DENOMINATOR_VALUE - fee_rate).into(),
                    )
                    .ok_or(ErrorCode::CalculateOverflow)?;
            }
        } else {
            result.fee_amount = result
                .amount_out
                .mul_div_ceil(fee_rate.into(), FEE_RATE_DENOMINATOR_VALUE.into())
                .ok_or(ErrorCode::CalculateOverflow)?;
            result.amount_out = result
                .amount_out
                .checked_sub(result.fee_amount)
                .ok_or(ErrorCode::CalculateOverflow)?;

            if !max {
                result.amount_in = amount_remaining;
            }
        }
    } else {
        if is_fee_on_input {
            result.amount_out = result.amount_out.min(amount_remaining);
            result.fee_amount = result
                .amount_in
                .mul_div_ceil(
                    fee_rate.into(),
                    (FEE_RATE_DENOMINATOR_VALUE - fee_rate).into(),
                )
                .ok_or(ErrorCode::CalculateOverflow)?;
        } else {
            result.fee_amount = result
                .amount_out
                .mul_div_ceil(fee_rate.into(), FEE_RATE_DENOMINATOR_VALUE.into())
                .ok_or(ErrorCode::CalculateOverflow)?;

            let net_output = result
                .amount_out
                .checked_sub(result.fee_amount)
                .ok_or(ErrorCode::CalculateOverflow)?;

            if net_output > amount_remaining {
                result.fee_amount = result
                    .amount_out
                    .checked_sub(amount_remaining)
                    .ok_or(ErrorCode::CalculateOverflow)?;
                result.amount_out = amount_remaining;
            } else {
                result.amount_out = net_output;
            }
        }
    }

    Ok(result)
}

fn calculate_amount_in_range(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    zero_for_one: bool,
    is_base_input: bool,
) -> Result<Option<u64>> {
    let result = if is_base_input {
        if zero_for_one {
            liquidity_math::get_delta_amount_0_unsigned(
                sqrt_price_target_x64,
                sqrt_price_current_x64,
                liquidity,
                true,
            )
        } else {
            liquidity_math::get_delta_amount_1_unsigned(
                sqrt_price_current_x64,
                sqrt_price_target_x64,
                liquidity,
                true,
            )
        }
    } else {
        if zero_for_one {
            liquidity_math::get_delta_amount_1_unsigned(
                sqrt_price_target_x64,
                sqrt_price_current_x64,
                liquidity,
                false,
            )
        } else {
            liquidity_math::get_delta_amount_0_unsigned(
                sqrt_price_current_x64,
                sqrt_price_target_x64,
                liquidity,
                false,
            )
        }
    };

    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e == ErrorCode::MaxTokenOverflow.into() => Ok(None),
        Err(_) => Err(ErrorCode::SqrtPriceLimitOverflow.into()),
    }
}

#[cfg(test)]
mod swap_math_test {
    use super::*;
    use crate::libraries::tick_math;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn compute_swap_step_test(
            sqrt_price_current_x64 in tick_math::MIN_SQRT_PRICE_X64..tick_math::MAX_SQRT_PRICE_X64,
            sqrt_price_target_x64 in tick_math::MIN_SQRT_PRICE_X64..tick_math::MAX_SQRT_PRICE_X64,
            liquidity in 1..u32::MAX as u128,
            amount_remaining in 1..u64::MAX,
            fee_rate in 1..(FEE_RATE_DENOMINATOR_VALUE - 1000),
            is_base_input in proptest::bool::ANY,
            is_fee_on_input in proptest::bool::ANY,
        ) {
            prop_assume!(sqrt_price_current_x64 != sqrt_price_target_x64);

            let price_diff = if sqrt_price_current_x64 > sqrt_price_target_x64 {
                sqrt_price_current_x64 - sqrt_price_target_x64
            } else {
                sqrt_price_target_x64 - sqrt_price_current_x64
            };
            prop_assume!(price_diff < u128::MAX / 1000);

            if !is_base_input && !is_fee_on_input {
                prop_assume!(amount_remaining <= u64::MAX / u64::from(FEE_RATE_DENOMINATOR_VALUE));
            }

            if !is_base_input && is_fee_on_input {
                prop_assume!(amount_remaining <= 1_000_000_000u64);
                prop_assume!(fee_rate <= FEE_RATE_DENOMINATOR_VALUE - 100);
            }

            let zero_for_one = sqrt_price_current_x64 > sqrt_price_target_x64;
            let swap_step = compute_swap(
                sqrt_price_current_x64,
                sqrt_price_target_x64,
                liquidity,
                amount_remaining,
                fee_rate,
                is_base_input,
                zero_for_one,
                is_fee_on_input,
            ).unwrap();

            let amount_in = swap_step.amount_in;
            let amount_out = swap_step.amount_out;
            let sqrt_price_next_x64 = swap_step.sqrt_price_next_x64;
            let fee_amount = swap_step.fee_amount;

            let amount_used = if is_base_input {
                if is_fee_on_input { amount_in + fee_amount } else { amount_in }
            } else {
                if is_fee_on_input { amount_out } else { amount_out + fee_amount }
            };

            if sqrt_price_next_x64 != sqrt_price_target_x64 {
                assert!(amount_used == amount_remaining);
            } else {
                assert!(amount_used <= amount_remaining);
            }
            let price_lower = sqrt_price_current_x64.min(sqrt_price_target_x64);
            let price_upper = sqrt_price_current_x64.max(sqrt_price_target_x64);
            assert!(sqrt_price_next_x64 >= price_lower);
            assert!(sqrt_price_next_x64 <= price_upper);
        }
    }
}
