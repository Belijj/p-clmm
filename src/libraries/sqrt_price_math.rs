use super::full_math::MulDiv;
use super::unsafe_math::UnsafeMathTrait;
use super::{fixed_point_64, U256};
use crate::error::ErrorCode;
use crate::{Result, require_gt};

pub fn get_next_sqrt_price_from_amount_0_rounding_up(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price_x64);
    };
    let numerator_1 = U256::from(liquidity) << fixed_point_64::RESOLUTION;

    if add {
        if let Some(product) = U256::from(amount).checked_mul(U256::from(sqrt_price_x64)) {
            let denominator = numerator_1 + product;
            if denominator >= numerator_1 {
                return Ok(numerator_1
                    .mul_div_ceil(U256::from(sqrt_price_x64), denominator)
                    .ok_or(ErrorCode::CalculateOverflow)?
                    .as_u128());
            };
        }

        Ok(U256::div_rounding_up(
            numerator_1,
            (numerator_1 / U256::from(sqrt_price_x64))
                .checked_add(U256::from(amount))
                .ok_or(ErrorCode::CalculateOverflow)?,
        )
        .as_u128())
    } else {
        let product = U256::from(amount)
            .checked_mul(U256::from(sqrt_price_x64))
            .ok_or(ErrorCode::CalculateOverflow)?;
        let denominator = numerator_1
            .checked_sub(product)
            .ok_or(ErrorCode::CalculateOverflow)?;
        Ok(numerator_1
            .mul_div_ceil(U256::from(sqrt_price_x64), denominator)
            .ok_or(ErrorCode::CalculateOverflow)?
            .as_u128())
    }
}

pub fn get_next_sqrt_price_from_amount_1_rounding_down(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price_x64);
    }

    if add {
        let quotient = (U256::from(amount) << fixed_point_64::RESOLUTION) / liquidity;
        sqrt_price_x64
            .checked_add(quotient.as_u128())
            .ok_or(ErrorCode::CalculateOverflow.into())
    } else {
        let quotient = U256::div_rounding_up(
            U256::from(u128::from(amount) << fixed_point_64::RESOLUTION),
            U256::from(liquidity),
        );
        sqrt_price_x64
            .checked_sub(quotient.as_u128())
            .ok_or(ErrorCode::CalculateOverflow.into())
    }
}

pub fn get_next_sqrt_price_from_input(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount_in: u64,
    zero_for_one: bool,
) -> Result<u128> {
    require_gt!(sqrt_price_x64, 0, ErrorCode::ZeroSqrtPrice);
    require_gt!(liquidity, 0, ErrorCode::ZeroLiquidity);

    if zero_for_one {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price_x64, liquidity, amount_in, true)
    } else {
        get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price_x64, liquidity, amount_in, true)
    }
}

pub fn get_next_sqrt_price_from_output(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount_out: u64,
    zero_for_one: bool,
) -> Result<u128> {
    require_gt!(sqrt_price_x64, 0, ErrorCode::ZeroSqrtPrice);
    require_gt!(liquidity, 0, ErrorCode::ZeroLiquidity);

    if zero_for_one {
        get_next_sqrt_price_from_amount_1_rounding_down(
            sqrt_price_x64,
            liquidity,
            amount_out,
            false,
        )
    } else {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price_x64, liquidity, amount_out, false)
    }
}
