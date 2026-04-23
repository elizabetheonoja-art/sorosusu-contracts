use soroban_sdk::{contractimpl, Address, Env, String, Symbol, Map};

#[derive(Clone)]
pub struct GoalEscrow {
    pub winner: Address,
    pub group_id: u32,
    pub amount: u128,
    pub asset: Address,
    pub vendor: Address,
    pub invoice_reference: String,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub delivery_confirmed_at: Option<u64>,
}

#[derive(Clone, PartialEq)]
pub enum EscrowStatus {
    PendingInvoice,
    AwaitingDelivery,
    Delivered,
    Cancelled,
}

pub trait GoalEscrowTrait {
    /// Create a goal-based escrow for a Susu winner
    fn create_goal_escrow(
        env: Env,
        group_id: u32,
        winner: Address,
        amount: u128,
        asset: Address,
        vendor: Address,
        invoice_reference: String,
    );

    /// Winner confirms delivery → funds released to vendor
    fn confirm_delivery(env: Env, escrow_id: u32);

    /// Get details of a specific goal escrow
    fn get_goal_escrow(env: Env, escrow_id: u32) -> GoalEscrow;

    /// Cancel escrow (only group admin or after timeout)
    fn cancel_goal_escrow(env: Env, escrow_id: u32);
}

#[contractimpl]
impl GoalEscrowTrait for SorosusuContract {
    fn create_goal_escrow(
        env: Env,
        group_id: u32,
        winner: Address,
        amount: u128,
        asset: Address,
        vendor: Address,
        invoice_reference: String,
    ) {
        // Authorization: Only group admin or winner can initiate
        // ... existing auth logic ...

        let escrow_id = Self::next_escrow_id(&env);

        let escrow = GoalEscrow {
            winner: winner.clone(),
            group_id,
            amount,
            asset: asset.clone(),
            vendor: vendor.clone(),
            invoice_reference,
            status: EscrowStatus::AwaitingDelivery,
            created_at: env.ledger().timestamp(),
            delivery_confirmed_at: None,
        };

        // Transfer funds from contract (or group pot) to escrow
        Self::transfer_to_escrow(&env, &asset, amount);

        Self::save_escrow(&env, escrow_id, escrow);

        env.events().publish(
            (Symbol::new(&env, "goal_escrow_created"),),
            (escrow_id, winner, amount, vendor)
        );
    }

    fn confirm_delivery(env: Env, escrow_id: u32) {
        let mut escrow = Self::get_goal_escrow(env.clone(), escrow_id);

        // Only winner can confirm delivery
        escrow.winner.require_auth();

        if escrow.status != EscrowStatus::AwaitingDelivery {
            panic_with_error!(&env, ContractError::InvalidEscrowState);
        }

        // Release funds to vendor
        let token_client = soroban_sdk::token::Client::new(&env, &escrow.asset);
        token_client.transfer(&env.current_contract_address(), &escrow.vendor, &(escrow.amount as i128));

        escrow.status = EscrowStatus::Delivered;
        escrow.delivery_confirmed_at = Some(env.ledger().timestamp());

        Self::save_escrow(&env, escrow_id, escrow);

        env.events().publish(
            (Symbol::new(&env, "goal_escrow_delivered"),),
            (escrow_id, escrow.vendor, escrow.amount)
        );
    }
}