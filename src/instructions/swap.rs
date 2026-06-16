
use crate::states::{AmmConfig, ObservationState, PoolState, TickArrayState};
use crate::util::{load, load_mut};
use core::array;
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub struct SwapArgs {
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: u128,
    pub is_base_input: bool,
}

impl SwapArgs {
    pub const SIZE: usize = 8 + 8 + 16 + 1;

    #[inline(always)]
    pub fn try_from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::SIZE {
            return Err(ProgramError::InvalidInstructionData);
        }
        let p = data.as_ptr();
        let amount = u64::from_le_bytes(unsafe { *(p as *const [u8; 8]) });
        let other_amount_threshold =
            u64::from_le_bytes(unsafe { *(p.add(8) as *const [u8; 8]) });
        let sqrt_price_limit_x64 =
            u128::from_le_bytes(unsafe { *(p.add(16) as *const [u8; 16]) });
        let is_base_input = data[32] != 0;
        Ok(Self {
            amount,
            other_amount_threshold,
            sqrt_price_limit_x64,
            is_base_input,
        })
    }
}

#[repr(usize)]
#[allow(dead_code)]
pub enum AccountIdx {
    Payer = 0,
    AmmConfig = 1,
    PoolState = 2,
    InputTokenAccount = 3,
    OutputTokenAccount = 4,
    InputVault = 5,
    OutputVault = 6,
    ObservationState = 7,
    TokenProgram = 8,
    TickArray = 9,
}

const MIN_ACCOUNTS: usize = AccountIdx::TickArray as usize + 1;

pub const MAX_TICK_ARRAYS_PER_SWAP: usize = 8;

pub struct TickArrayPath<'a> {
    entries: [Option<&'a mut TickArrayState>; MAX_TICK_ARRAYS_PER_SWAP],
    len: usize,
}

impl<'a> TickArrayPath<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: array::from_fn(|_| None),
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, t: &'a mut TickArrayState) -> Result<(), ProgramError> {
        if self.len >= MAX_TICK_ARRAYS_PER_SWAP {
            return Err(ProgramError::InvalidArgument);
        }
        self.entries[self.len] = Some(t);
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<&'a mut TickArrayState> {
        if self.len == 0 {
            return None;
        }
        let head = self.entries[0].take();
        for i in 0..self.len - 1 {
            self.entries[i] = self.entries[i + 1].take();
        }
        self.len -= 1;
        head
    }
}

pub struct SwapAccounts<'a> {
    pub payer: &'a AccountInfo,
    pub amm_config: &'a AmmConfig,
    pub pool_state_info: &'a AccountInfo,
    pub pool_state: &'a mut PoolState,
    pub input_token_account: &'a AccountInfo,
    pub output_token_account: &'a AccountInfo,
    pub input_vault: &'a AccountInfo,
    pub output_vault: &'a AccountInfo,
    pub observation_state: &'a mut ObservationState,
    pub token_program: &'a AccountInfo,
    pub tick_arrays: TickArrayPath<'a>,
}

impl<'a> SwapAccounts<'a> {
    pub fn try_bind(
        accounts: &'a [AccountInfo],
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        if accounts.len() < MIN_ACCOUNTS {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let payer = &accounts[AccountIdx::Payer as usize];
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let amm_config = load::<AmmConfig>(
            &accounts[AccountIdx::AmmConfig as usize],
            program_id,
        )?;

        let pool_state_info = &accounts[AccountIdx::PoolState as usize];
        let pool_state = load_mut::<PoolState>(pool_state_info, program_id)?;

        let expected_amm_config: Pubkey = pool_state.amm_config;
        if accounts[AccountIdx::AmmConfig as usize].key() != &expected_amm_config {
            return Err(ProgramError::InvalidArgument);
        }

        let input_token_account = &accounts[AccountIdx::InputTokenAccount as usize];
        let output_token_account = &accounts[AccountIdx::OutputTokenAccount as usize];
        let input_vault = &accounts[AccountIdx::InputVault as usize];
        let output_vault = &accounts[AccountIdx::OutputVault as usize];

        if !input_vault.is_writable() || !output_vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !input_token_account.is_writable() || !output_token_account.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        let observation_state = load_mut::<ObservationState>(
            &accounts[AccountIdx::ObservationState as usize],
            program_id,
        )?;
        let expected_observation_key: Pubkey = pool_state.observation_key;
        if accounts[AccountIdx::ObservationState as usize].key() != &expected_observation_key {
            return Err(ProgramError::InvalidArgument);
        }

        let token_program = &accounts[AccountIdx::TokenProgram as usize];

        let pool_key: Pubkey = *accounts[AccountIdx::PoolState as usize].key();
        let mut tick_arrays = TickArrayPath::new();

        let first_array = load_mut::<TickArrayState>(
            &accounts[AccountIdx::TickArray as usize],
            program_id,
        )?;
        let first_pool_id: Pubkey = first_array.pool_id;
        if first_pool_id != pool_key {
            return Err(ProgramError::InvalidArgument);
        }
        tick_arrays.push(first_array)?;

        for info in &accounts[AccountIdx::TickArray as usize + 1..] {
            let array = load_mut::<TickArrayState>(info, program_id)?;
            let array_pool_id: Pubkey = array.pool_id;
            if array_pool_id != pool_key {
                return Err(ProgramError::InvalidArgument);
            }
            tick_arrays.push(array)?;
        }

        Ok(Self {
            payer,
            amm_config,
            pool_state_info,
            pool_state,
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            observation_state,
            token_program,
            tick_arrays,
        })
    }
}

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let args = SwapArgs::try_from_bytes(data)?;
    let mut bound = SwapAccounts::try_bind(accounts, program_id)?;

    use pinocchio::sysvars::{clock::Clock, Sysvar};
    let block_timestamp = Clock::get()
        .map(|c| c.unix_timestamp as u64)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    crate::instructions::exact_internal::exact_internal(
        &mut bound,
        program_id,
        args.amount,
        args.other_amount_threshold,
        args.sqrt_price_limit_x64,
        args.is_base_input,
        block_timestamp,
    )
    .map(|_| ())
    .map_err(Into::into)
}

#[cfg(test)]
mod args_tests {
    use super::*;

    fn pack(amount: u64, threshold: u64, limit: u128, base_in: bool) -> [u8; 33] {
        let mut buf = [0u8; 33];
        buf[0..8].copy_from_slice(&amount.to_le_bytes());
        buf[8..16].copy_from_slice(&threshold.to_le_bytes());
        buf[16..32].copy_from_slice(&limit.to_le_bytes());
        buf[32] = base_in as u8;
        buf
    }

    #[test]
    fn roundtrip_basic() {
        let buf = pack(1_000, 500, 1u128 << 64, true);
        let args = SwapArgs::try_from_bytes(&buf).unwrap();
        assert_eq!(args.amount, 1_000);
        assert_eq!(args.other_amount_threshold, 500);
        assert_eq!(args.sqrt_price_limit_x64, 1u128 << 64);
        assert!(args.is_base_input);
    }

    #[test]
    fn rejects_short_buffer() {
        let buf = [0u8; 32];
        assert!(SwapArgs::try_from_bytes(&buf).is_err());
    }

    #[test]
    fn accepts_extra_bytes() {
        let mut buf = [0u8; 64];
        buf[0..8].copy_from_slice(&42u64.to_le_bytes());
        let args = SwapArgs::try_from_bytes(&buf).unwrap();
        assert_eq!(args.amount, 42);
    }

    #[test]
    fn is_base_input_non_zero_byte() {
        let buf = pack(1, 1, 1, false);
        assert!(!SwapArgs::try_from_bytes(&buf).unwrap().is_base_input);
        let mut buf = pack(1, 1, 1, true);
        buf[32] = 7;
        assert!(SwapArgs::try_from_bytes(&buf).unwrap().is_base_input);
    }

    #[test]
    fn extreme_values_roundtrip() {
        let buf = pack(u64::MAX, u64::MAX, u128::MAX, true);
        let args = SwapArgs::try_from_bytes(&buf).unwrap();
        assert_eq!(args.amount, u64::MAX);
        assert_eq!(args.other_amount_threshold, u64::MAX);
        assert_eq!(args.sqrt_price_limit_x64, u128::MAX);
    }
}
