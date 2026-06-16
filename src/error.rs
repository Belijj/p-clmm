use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    NotApproved = 6000,
    InvalidUpdateConfigFlag,
    AccountLack,
    ClosePositionErr,
    ZeroMintAmount,
    InvalidTickIndex,
    TickInvalidOrder,
    TickLowerOverflow,
    TickUpperOverflow,
    TickAndSpacingNotMatch,
    InvalidTickArray,
    InvalidTickArrayBoundary,
    SqrtPriceLimitOverflow,
    SqrtPriceX64,
    LiquiditySubValueErr,
    LiquidityAddValueErr,
    InvalidLiquidity,
    ForbidBothZeroForSupplyLiquidity,
    LiquidityInsufficient,
    TransactionTooOld,
    PriceSlippageCheck,
    TooLittleOutputReceived,
    TooMuchInputPaid,
    InvaildSwapAmountSpecified,
    InvalidInputPoolVault,
    TooSmallInputOrOutputAmount,
    NotEnoughTickArrayAccount,
    InvalidFirstTickArrayAccount,
    InvalidRewardIndex,
    FullRewardInfo,
    RewardTokenAlreadyInUse,
    ExceptPoolVaultMint,
    InvalidRewardInitParam,
    InvalidRewardDesiredAmount,
    InvalidRewardInputAccountNumber,
    InvalidRewardPeriod,
    NotApproveUpdateRewardEmissiones,
    UnInitializedRewardInfo,
    NotSupportMint,
    MissingTickArrayBitmapExtensionAccount,
    InsufficientLiquidityForDirection,
    MaxTokenOverflow,
    CalculateOverflow,
    TransferFeeCalculateNotMatch,
    MissingBaseFlag,
    ZeroSqrtPrice,
    ZeroLiquidity,
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}
