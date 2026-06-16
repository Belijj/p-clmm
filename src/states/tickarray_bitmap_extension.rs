use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

pub const EXTENSION_TICKARRAY_BITMAP_SIZE: usize = 14;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TickArrayBitmapExtension {
    pub pool_id: Pubkey,
    pub positive_tick_array_bitmap: [[u64; 8]; EXTENSION_TICKARRAY_BITMAP_SIZE],
    pub negative_tick_array_bitmap: [[u64; 8]; EXTENSION_TICKARRAY_BITMAP_SIZE],
}

impl crate::util::AccountSchema for TickArrayBitmapExtension {
    const DISCRIMINATOR: [u8; 8] = crate::discriminator::account::TICK_ARRAY_BITMAP_EXTENSION;
}

impl TickArrayBitmapExtension {
    pub const DISCRIMINATOR: [u8; 8] =
        crate::discriminator::account::TICK_ARRAY_BITMAP_EXTENSION;

    pub const LEN: usize = 8 + 32 + 64 * EXTENSION_TICKARRAY_BITMAP_SIZE * 2;
}
