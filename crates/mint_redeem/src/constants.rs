/// Template ID for DepositAccount contracts
pub const DEPOSIT_ACCOUNT_TEMPLATE_ID: &str = "#cbtc:CBTC.DepositAccount:CBTCDepositAccount";

/// Template ID for DepositAccountRules contracts
pub const DEPOSIT_ACCOUNT_RULES_TEMPLATE_ID: &str =
    "#cbtc:CBTC.DepositAccountRules:CBTCDepositAccountRules";

/// Template ID for DepositRequest contracts
pub const DEPOSIT_REQUEST_TEMPLATE_ID: &str = "#cbtc:CBTC.DepositRequest:CBTCDepositRequest";

/// Template ID for WithdrawAccount contracts
pub const WITHDRAW_ACCOUNT_TEMPLATE_ID: &str = "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccount";

/// Template ID for WithdrawAccountRules contracts
pub const WITHDRAW_ACCOUNT_RULES_TEMPLATE_ID: &str =
    "#cbtc:CBTC.WithdrawAccountRules:CBTCWithdrawAccountRules";

/// Template ID for WithdrawRequest contracts
pub const WITHDRAW_REQUEST_TEMPLATE_ID: &str = "#cbtc:CBTC.WithdrawRequest:CBTCWithdrawRequest";

/// Choice name for creating a deposit account
pub const CREATE_DEPOSIT_ACCOUNT_CHOICE: &str = "CBTCDepositAccountRules_CreateDepositAccount";

/// Choice name for creating a withdraw account
pub const CREATE_WITHDRAW_ACCOUNT_CHOICE: &str = "CBTCWithdrawAccountRules_CreateWithdrawAccount";

/// Choice name for withdrawing (burning) CBTC
pub const WITHDRAW_CHOICE: &str = "CBTCWithdrawAccount_Withdraw";
