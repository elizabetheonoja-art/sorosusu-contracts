use soroban_sdk::{contractimpl, Address, Env, String, Symbol};

#[derive(Clone)]
pub struct ContributionReceipt {
    pub contributor: Address,
    pub group_id: u32,
    pub amount: u128,
    pub asset_code: String,
    pub timestamp: u64,
}

pub trait ReceiptTrait {
    /// Generates a localized-friendly receipt string
    /// Example output: "Confirmed: You contributed 50 USDC to Group Savings X"
    fn generate_receipt(
        env: Env,
        contributor: Address,
        group_id: u32,
        amount: u128,
        asset_code: String,
        group_name: String,
    ) -> String;
}

#[contractimpl]
impl ReceiptTrait for SorosusuContract {
    fn generate_receipt(
        env: Env,
        contributor: Address,
        group_id: u32,
        amount: u128,
        asset_code: String,
        group_name: String,
    ) -> String {
        // Format: "Confirmed: You contributed 50 USDC to Group X"
        let mut receipt = String::from_str(&env, "Confirmed: You contributed ");

        // Append amount
        receipt = receipt.concat(&String::from_str(&env, &amount.to_string()));
        receipt = receipt.concat(&String::from_str(&env, " "));

        // Append asset code
        receipt = receipt.concat(&asset_code);

        // Append group info
        receipt = receipt.concat(&String::from_str(&env, " to "));
        receipt = receipt.concat(&group_name);

        // Add timestamp for auditability
        let timestamp_str = String::from_str(&env, " at ");
        receipt = receipt.concat(&timestamp_str);
        receipt = receipt.concat(&String::from_str(&env, &env.ledger().timestamp().to_string()));

        receipt
    }
}