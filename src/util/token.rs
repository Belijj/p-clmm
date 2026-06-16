
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Signer},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

pub const SPL_TOKEN_ID: Pubkey =
    pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const TOKEN_2022_ID: Pubkey =
    pinocchio_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const P_TOKEN_ID: Pubkey =
    pinocchio_pubkey::pubkey!("pTokenrcXyfwwrHaitJwGsuMEcZZbywxK7DkAaKZTms");

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TokenKind {
    SplClassic,
    Token2022,
    PToken,
}

impl TokenKind {
    #[inline(always)]
    pub fn from_owner(owner: &Pubkey) -> Result<Self, ProgramError> {
        if *owner == SPL_TOKEN_ID {
            Ok(Self::SplClassic)
        } else if *owner == TOKEN_2022_ID {
            Ok(Self::Token2022)
        } else if *owner == P_TOKEN_ID {
            Ok(Self::PToken)
        } else {
            Err(ProgramError::IncorrectProgramId)
        }
    }

    #[inline(always)]
    pub fn program_id(self) -> &'static Pubkey {
        match self {
            Self::SplClassic => &SPL_TOKEN_ID,
            Self::Token2022 => &TOKEN_2022_ID,
            Self::PToken => &P_TOKEN_ID,
        }
    }
}

#[inline]
pub fn transfer_checked(
    kind: TokenKind,
    token_program: &AccountInfo,
    from: &AccountInfo,
    mint: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    decimals: u8,
    signer_seeds: &[Signer],
) -> ProgramResult {
    if token_program.key() != kind.program_id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut data = [0u8; 10];
    data[0] = 12;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data[9] = decimals;

    let metas = [
        AccountMeta::writable(from.key()),
        AccountMeta::readonly(mint.key()),
        AccountMeta::writable(to.key()),
        AccountMeta::readonly_signer(authority.key()),
    ];

    let ix = Instruction {
        program_id: kind.program_id(),
        accounts: &metas,
        data: &data,
    };

    invoke_signed(&ix, &[from, mint, to, authority], signer_seeds)
}

const ACCOUNT_TYPE_MINT: u8 = 1;
const MINT_BASE_LEN: usize = 82;
const EXT_TYPE_TRANSFER_FEE_CONFIG: u16 = 1;

const TFEE_OLDER_OFFSET: usize = 72;
const TFEE_NEWER_OFFSET: usize = 96;
const TFEE_ENTRY_LEN: usize = 24;

#[inline]
pub fn find_transfer_fee(mint_owner: &Pubkey, mint_data: &[u8], epoch: u64) -> Option<TransferFee> {
    if *mint_owner != TOKEN_2022_ID {
        return None;
    }
    if mint_data.len() <= MINT_BASE_LEN || mint_data[MINT_BASE_LEN] != ACCOUNT_TYPE_MINT {
        return None;
    }
    let mut cursor = MINT_BASE_LEN + 1;
    while cursor + 4 <= mint_data.len() {
        let ty = u16::from_le_bytes([mint_data[cursor], mint_data[cursor + 1]]);
        let len =
            u16::from_le_bytes([mint_data[cursor + 2], mint_data[cursor + 3]]) as usize;
        cursor += 4;
        if cursor + len > mint_data.len() {
            return None;
        }
        if ty == EXT_TYPE_TRANSFER_FEE_CONFIG && len >= TFEE_NEWER_OFFSET + TFEE_ENTRY_LEN {
            let older = parse_fee_entry(&mint_data[cursor + TFEE_OLDER_OFFSET..]);
            let newer = parse_fee_entry(&mint_data[cursor + TFEE_NEWER_OFFSET..]);
            return Some(if epoch >= newer.epoch { newer } else { older });
        }
        cursor += len;
    }
    None
}

#[derive(Copy, Clone, Debug)]
pub struct TransferFee {
    pub epoch: u64,
    pub maximum_fee: u64,
    pub transfer_fee_basis_points: u16,
}

#[inline]
fn parse_fee_entry(slice: &[u8]) -> TransferFee {
    let epoch = u64::from_le_bytes(slice[0..8].try_into().unwrap());
    let maximum_fee = u64::from_le_bytes(slice[8..16].try_into().unwrap());
    let transfer_fee_basis_points = u16::from_le_bytes([slice[16], slice[17]]);
    TransferFee {
        epoch,
        maximum_fee,
        transfer_fee_basis_points,
    }
}

#[inline]
pub fn calculate_transfer_fee(fee: &TransferFee, gross_amount: u64) -> u64 {
    if fee.transfer_fee_basis_points == 0 || gross_amount == 0 {
        return 0;
    }
    let numerator = (gross_amount as u128) * (fee.transfer_fee_basis_points as u128);
    let raw = numerator.div_ceil(10_000u128) as u64;
    core::cmp::min(raw, fee.maximum_fee)
}

#[inline]
pub fn calculate_pre_fee_amount(fee: &TransferFee, post_fee_amount: u64) -> Option<u64> {
    if fee.transfer_fee_basis_points == 0 {
        return Some(post_fee_amount);
    }
    if fee.transfer_fee_basis_points as u32 >= 10_000 {
        return None;
    }
    let denominator = 10_000u128 - fee.transfer_fee_basis_points as u128;
    let numerator = (post_fee_amount as u128) * 10_000u128;
    let raw = numerator.div_ceil(denominator);
    if raw > u64::MAX as u128 {
        None
    } else {
        Some(raw as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_zero_bps_is_noop() {
        let f = TransferFee { epoch: 0, maximum_fee: 100, transfer_fee_basis_points: 0 };
        assert_eq!(calculate_transfer_fee(&f, 1_000_000), 0);
        assert_eq!(calculate_pre_fee_amount(&f, 1_000_000), Some(1_000_000));
    }

    #[test]
    fn fee_capped_at_maximum() {
        let f = TransferFee { epoch: 0, maximum_fee: 50, transfer_fee_basis_points: 5_000 };
        assert_eq!(calculate_transfer_fee(&f, 1_000), 50);
    }

    #[test]
    fn fee_roundtrip_within_bps_range() {
        let f = TransferFee { epoch: 0, maximum_fee: u64::MAX, transfer_fee_basis_points: 250 };
        for gross in [1_000u64, 1_000_000, 999_999, u32::MAX as u64] {
            let fee = calculate_transfer_fee(&f, gross);
            let net = gross - fee;
            let recovered = calculate_pre_fee_amount(&f, net).unwrap();
            assert!(recovered >= gross.saturating_sub(1) && recovered <= gross + 1);
        }
    }

    #[test]
    fn find_fee_returns_none_for_classic_mint() {
        let fee = find_transfer_fee(&SPL_TOKEN_ID, &[0u8; 200], 100);
        assert!(fee.is_none());
    }

    #[test]
    fn find_fee_returns_none_for_t22_mint_without_extension() {
        let mut data = [0u8; MINT_BASE_LEN + 1];
        data[MINT_BASE_LEN] = ACCOUNT_TYPE_MINT;
        let fee = find_transfer_fee(&TOKEN_2022_ID, &data, 100);
        assert!(fee.is_none());
    }

    #[test]
    fn fee_basis_points_at_full_bps() {
        let f = TransferFee { epoch: 0, maximum_fee: u64::MAX, transfer_fee_basis_points: 10_000 };
        assert_eq!(calculate_transfer_fee(&f, 1_000), 1_000);
        assert!(calculate_pre_fee_amount(&f, 100).is_none());
    }

    #[test]
    fn fee_at_9999_bps_inverse_finite() {
        let f = TransferFee { epoch: 0, maximum_fee: u64::MAX, transfer_fee_basis_points: 9_999 };
        let pre = calculate_pre_fee_amount(&f, 1).expect("should be Some");
        assert!(pre >= 10_000);
    }

    #[test]
    fn token_kind_classifies_each_program() {
        assert_eq!(TokenKind::from_owner(&SPL_TOKEN_ID).unwrap(), TokenKind::SplClassic);
        assert_eq!(TokenKind::from_owner(&TOKEN_2022_ID).unwrap(), TokenKind::Token2022);
        assert_eq!(TokenKind::from_owner(&P_TOKEN_ID).unwrap(), TokenKind::PToken);
        assert!(TokenKind::from_owner(&[0u8; 32]).is_err());
    }

    #[test]
    fn token_kind_program_id_roundtrip() {
        for kind in [TokenKind::SplClassic, TokenKind::Token2022, TokenKind::PToken] {
            let id = kind.program_id();
            let back = TokenKind::from_owner(id).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn find_fee_walks_past_unrelated_extensions() {
        const TOTAL: usize = MINT_BASE_LEN + 1 + 4 + 8 + 4 + TFEE_NEWER_OFFSET + TFEE_ENTRY_LEN;
        let mut data = [0u8; TOTAL];
        data[MINT_BASE_LEN] = ACCOUNT_TYPE_MINT;
        let mut c = MINT_BASE_LEN + 1;
        data[c..c + 2].copy_from_slice(&99u16.to_le_bytes());
        data[c + 2..c + 4].copy_from_slice(&8u16.to_le_bytes());
        c += 4 + 8;
        data[c..c + 2].copy_from_slice(&EXT_TYPE_TRANSFER_FEE_CONFIG.to_le_bytes());
        data[c + 2..c + 4].copy_from_slice(&((TFEE_NEWER_OFFSET + TFEE_ENTRY_LEN) as u16).to_le_bytes());
        c += 4;
        data[c + TFEE_NEWER_OFFSET + 16..c + TFEE_NEWER_OFFSET + 18]
            .copy_from_slice(&250u16.to_le_bytes());
        let fee = find_transfer_fee(&TOKEN_2022_ID, &data, 999).expect("should find fee");
        assert_eq!(fee.transfer_fee_basis_points, 250);
    }

    #[test]
    fn find_fee_rejects_truncated_tlv() {
        const TOTAL: usize = MINT_BASE_LEN + 1 + 4 + 5;
        let mut data = [0u8; TOTAL];
        data[MINT_BASE_LEN] = ACCOUNT_TYPE_MINT;
        let c = MINT_BASE_LEN + 1;
        data[c..c + 2].copy_from_slice(&EXT_TYPE_TRANSFER_FEE_CONFIG.to_le_bytes());
        data[c + 2..c + 4].copy_from_slice(&999u16.to_le_bytes());
        assert!(find_transfer_fee(&TOKEN_2022_ID, &data, 0).is_none());
    }

    #[test]
    fn fee_zero_amount_is_zero() {
        let f = TransferFee { epoch: 0, maximum_fee: u64::MAX, transfer_fee_basis_points: 500 };
        assert_eq!(calculate_transfer_fee(&f, 0), 0);
    }
}
