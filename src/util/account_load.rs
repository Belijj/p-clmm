
use crate::Result;
use core::mem;
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey,
};

pub trait AccountSchema: bytemuck::Pod + bytemuck::Zeroable {
    const DISCRIMINATOR: [u8; 8];
}

#[inline(always)]
unsafe fn validate<T: AccountSchema>(
    info: &AccountInfo,
    program_id: &Pubkey,
    require_writable: bool,
) -> Result<*mut u8> {
    if info.owner() != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if require_writable && !info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    if info.data_len() < 8 + mem::size_of::<T>() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let data: &[u8] = unsafe { info.borrow_data_unchecked() };
    let disc: [u8; 8] = unsafe { *(data.as_ptr() as *const [u8; 8]) };
    if disc != T::DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(data.as_ptr() as *mut u8)
}

#[inline(always)]
pub fn load<'a, T: AccountSchema>(
    info: &'a AccountInfo,
    program_id: &Pubkey,
) -> Result<&'a T> {
    let data_ptr = unsafe { validate::<T>(info, program_id, false)? };
    Ok(unsafe { &*(data_ptr.add(8) as *const T) })
}

#[inline(always)]
pub fn load_mut<'a, T: AccountSchema>(
    info: &'a AccountInfo,
    program_id: &Pubkey,
) -> Result<&'a mut T> {
    let data_ptr = unsafe { validate::<T>(info, program_id, true)? };
    Ok(unsafe { &mut *(data_ptr.add(8) as *mut T) })
}

#[inline(always)]
pub fn load_init<'a, T: AccountSchema>(
    info: &'a AccountInfo,
    program_id: &Pubkey,
) -> Result<&'a mut T> {
    if info.owner() != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    if info.data_len() < 8 + mem::size_of::<T>() {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let data: &mut [u8] = unsafe { info.borrow_mut_data_unchecked() };
    let data_ptr = data.as_mut_ptr();
    let disc: [u8; 8] = unsafe { *(data_ptr as *const [u8; 8]) };
    if disc != [0; 8] {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    unsafe {
        *(data_ptr as *mut [u8; 8]) = T::DISCRIMINATOR;
    }
    Ok(unsafe { &mut *(data_ptr.add(8) as *mut T) })
}
