
use crate::error::ErrorCode;
use crate::instructions::swap::SwapAccounts;
use crate::instructions::swap_internal::{swap_internal, SwapInternalResult};
use crate::states::POOL_SEED;
use crate::util::token::{transfer_checked, TokenKind};
use crate::{require, Result};
use pinocchio::{
    instruction::{Seed, Signer},
    pubkey::Pubkey,
};

const VAULT_AMOUNT_OFFSET: usize = 64;

#[inline(always)]
fn read_vault_amount(info: &pinocchio::account_info::AccountInfo) -> u64 {
    if info.data_len() < VAULT_AMOUNT_OFFSET + 8 {
        return 0;
    }
    let data = unsafe { info.borrow_data_unchecked() };
    u64::from_le_bytes(unsafe {
        *(data.as_ptr().add(VAULT_AMOUNT_OFFSET) as *const [u8; 8])
    })
}

pub fn exact_internal<'a>(
    ctx: &mut SwapAccounts<'a>,
    _program_id: &Pubkey,
    amount_specified: u64,
    other_amount_threshold: u64,
    sqrt_price_limit_x64: u128,
    is_base_input: bool,
    block_timestamp: u64,
) -> Result<u64> {
    let input_vault_mint_bytes: [u8; 32] = read_vault_mint(ctx.input_vault);
    let token_mint_0_bytes: Pubkey = ctx.pool_state.token_mint_0;
    let zero_for_one = input_vault_mint_bytes == token_mint_0_bytes;

    let pool_vault_0: Pubkey = ctx.pool_state.token_vault_0;
    let pool_vault_1: Pubkey = ctx.pool_state.token_vault_1;
    let (input_vault_expected, output_vault_expected) = if zero_for_one {
        (pool_vault_0, pool_vault_1)
    } else {
        (pool_vault_1, pool_vault_0)
    };
    if ctx.input_vault.key() != &input_vault_expected
        || ctx.output_vault.key() != &output_vault_expected
    {
        return Err(ErrorCode::InvalidInputPoolVault.into());
    }

    let (decimals_input, decimals_output) = if zero_for_one {
        (ctx.pool_state.mint_decimals_0, ctx.pool_state.mint_decimals_1)
    } else {
        (ctx.pool_state.mint_decimals_1, ctx.pool_state.mint_decimals_0)
    };

    let input_balance_before = read_vault_amount(ctx.input_vault);
    let output_balance_before = read_vault_amount(ctx.output_vault);

    let token_kind = TokenKind::from_owner(ctx.input_vault.owner())?;
    if ctx.output_vault.owner() != ctx.input_vault.owner() {
        return Err(ErrorCode::InvalidInputPoolVault.into());
    }

    let swap_result: SwapInternalResult = swap_internal(
        ctx.pool_state,
        &mut ctx.tick_arrays,
        ctx.amm_config,
        ctx.observation_state,
        amount_specified,
        if sqrt_price_limit_x64 == 0 {
            if zero_for_one {
                crate::libraries::tick_math::MIN_SQRT_PRICE_X64 + 1
            } else {
                crate::libraries::tick_math::MAX_SQRT_PRICE_X64 - 1
            }
        } else {
            sqrt_price_limit_x64
        },
        zero_for_one,
        is_base_input,
        block_timestamp,
    )?;

    require!(
        swap_result.amount_0 != 0 && swap_result.amount_1 != 0,
        ErrorCode::TooSmallInputOrOutputAmount
    );

    let (amount_in, amount_out) = if zero_for_one {
        (swap_result.amount_0, swap_result.amount_1)
    } else {
        (swap_result.amount_1, swap_result.amount_0)
    };

    let mint_for_input_transfer = if zero_for_one {
        ctx.input_vault
    } else {
        ctx.input_vault
    };
    let _ = mint_for_input_transfer;
    spl_classic_transfer(
        ctx.token_program,
        ctx.input_token_account,
        ctx.input_vault,
        ctx.payer,
        amount_in,
    )?;

    let pool_bump: u8 = ctx.pool_state.bump[0];
    let amm_config: Pubkey = ctx.pool_state.amm_config;
    let mint_0: Pubkey = ctx.pool_state.token_mint_0;
    let mint_1: Pubkey = ctx.pool_state.token_mint_1;
    let bump_arr = [pool_bump];

    let pool_seeds = [
        Seed::from(POOL_SEED),
        Seed::from(amm_config.as_ref()),
        Seed::from(mint_0.as_ref()),
        Seed::from(mint_1.as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    let signer = Signer::from(&pool_seeds);

    spl_classic_transfer_signed(
        ctx.token_program,
        ctx.output_vault,
        ctx.output_token_account,
        ctx.pool_state_info,
        amount_out,
        &[signer],
    )?;

    let _ = (
        input_balance_before,
        output_balance_before,
        token_kind,
        decimals_input,
        decimals_output,
    );
    if is_base_input {
        require!(amount_out >= other_amount_threshold, ErrorCode::TooLittleOutputReceived);
        Ok(amount_out)
    } else {
        require!(amount_in <= other_amount_threshold, ErrorCode::TooMuchInputPaid);
        Ok(amount_in)
    }
}

#[inline(always)]
fn read_vault_mint(info: &pinocchio::account_info::AccountInfo) -> [u8; 32] {
    if info.data_len() < 32 {
        return [0u8; 32];
    }
    let data = unsafe { info.borrow_data_unchecked() };
    let mut out = [0u8; 32];
    out.copy_from_slice(&data[0..32]);
    out
}

fn spl_classic_transfer(
    token_program: &pinocchio::account_info::AccountInfo,
    from: &pinocchio::account_info::AccountInfo,
    to: &pinocchio::account_info::AccountInfo,
    authority: &pinocchio::account_info::AccountInfo,
    amount: u64,
) -> Result<()> {
    use pinocchio::instruction::{AccountMeta, Instruction};
    use pinocchio::program::invoke;

    let mut data = [0u8; 9];
    data[0] = 3;
    data[1..9].copy_from_slice(&amount.to_le_bytes());

    let metas = [
        AccountMeta::writable(from.key()),
        AccountMeta::writable(to.key()),
        AccountMeta::readonly_signer(authority.key()),
    ];

    let ix = Instruction {
        program_id: token_program.key(),
        accounts: &metas,
        data: &data,
    };
    invoke(&ix, &[from, to, authority]).map_err(Into::into)
}

fn spl_classic_transfer_signed(
    token_program: &pinocchio::account_info::AccountInfo,
    from: &pinocchio::account_info::AccountInfo,
    to: &pinocchio::account_info::AccountInfo,
    authority: &pinocchio::account_info::AccountInfo,
    amount: u64,
    signers: &[Signer],
) -> Result<()> {
    use pinocchio::instruction::{AccountMeta, Instruction};
    use pinocchio::program::invoke_signed;

    let mut data = [0u8; 9];
    data[0] = 3;
    data[1..9].copy_from_slice(&amount.to_le_bytes());

    let metas = [
        AccountMeta::writable(from.key()),
        AccountMeta::writable(to.key()),
        AccountMeta::readonly_signer(authority.key()),
    ];

    let ix = Instruction {
        program_id: token_program.key(),
        accounts: &metas,
        data: &data,
    };
    invoke_signed(&ix, &[from, to, authority], signers).map_err(Into::into)
}
