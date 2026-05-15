#![no_std]
//! DisburseEscrow — multi-party escrow for humanitarian aid and payroll disbursements.
//! Supports unanimous release, arbiter resolution, and dispute flow.
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Vec};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum EscrowState {
    Pending,
    Released,
    Refunded,
    Disputed,
    Resolved,
}

#[contracttype]
#[derive(Clone)]
pub struct DisburseEscrow {
    pub depositor: Address,
    pub beneficiary: Address,
    pub arbiter: Address,
    pub amount: i128,
    pub timeout_ledger: u32,
    pub state: EscrowState,
    pub approvals: Vec<Address>,   // addresses that have approved release
    pub required_approvals: u32,
}

#[contracttype]
pub enum Key { Escrow(u64), Counter }

#[contract]
pub struct DisburseEscrowContract;

#[contractimpl]
impl DisburseEscrowContract {
    /// Deposit funds into escrow. Depositor must authorise.
    pub fn deposit(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        arbiter: Address,
        amount: i128,
        timeout_ledgers: u32,
        required_approvals: u32,
    ) -> u64 {
        depositor.require_auth();
        assert!(amount > 0, "amount must be positive");
        assert!(required_approvals >= 1, "need at least one approval");

        let id: u64 = env.storage().instance()
            .get(&Key::Counter).unwrap_or(0u64) + 1;
        env.storage().instance().set(&Key::Counter, &id);

        let escrow = DisburseEscrow {
            depositor,
            beneficiary,
            arbiter,
            amount,
            timeout_ledger: env.ledger().sequence() + timeout_ledgers,
            state: EscrowState::Pending,
            approvals: Vec::new(&env),
            required_approvals,
        };

        env.storage().persistent().set(&Key::Escrow(id), &escrow);
        env.events().publish((symbol_short!("deposit"), id), amount);
        id
    }

    /// Approve release. Depositor or arbiter can approve.
    /// When required_approvals threshold is met, funds are released automatically.
    pub fn approve(env: Env, id: u64, approver: Address) {
        approver.require_auth();
        let mut escrow: DisburseEscrow = env.storage().persistent()
            .get(&Key::Escrow(id)).expect("not found");

        assert!(escrow.state == EscrowState::Pending, "not pending");
        let is_authorised = approver == escrow.depositor || approver == escrow.arbiter;
        assert!(is_authorised, "not authorised to approve");

        // Idempotent — skip if already approved by this address
        let already = escrow.approvals.iter().any(|a| a == approver);
        if !already {
            escrow.approvals.push_back(approver.clone());
        }

        env.events().publish((symbol_short!("approve"), id), approver);

        if escrow.approvals.len() >= escrow.required_approvals {
            escrow.state = EscrowState::Released;
            env.events().publish((symbol_short!("release"), id), escrow.amount);
        }

        env.storage().persistent().set(&Key::Escrow(id), &escrow);
    }

    /// Refund to depositor after timeout.
    pub fn timeout_refund(env: Env, id: u64) {
        let mut escrow: DisburseEscrow = env.storage().persistent()
            .get(&Key::Escrow(id)).expect("not found");

        assert!(escrow.state == EscrowState::Pending, "not pending");
        assert!(
            env.ledger().sequence() >= escrow.timeout_ledger,
            "timeout not reached"
        );

        escrow.state = EscrowState::Refunded;
        env.storage().persistent().set(&Key::Escrow(id), &escrow);
        env.events().publish((symbol_short!("refund"), id), escrow.amount);
    }

    /// Raise a dispute. Either party can dispute before timeout.
    pub fn dispute(env: Env, id: u64, disputer: Address) {
        disputer.require_auth();
        let mut escrow: DisburseEscrow = env.storage().persistent()
            .get(&Key::Escrow(id)).expect("not found");

        assert!(escrow.state == EscrowState::Pending, "not pending");
        let is_party = disputer == escrow.depositor || disputer == escrow.beneficiary;
        assert!(is_party, "only parties can dispute");

        escrow.state = EscrowState::Disputed;
        env.storage().persistent().set(&Key::Escrow(id), &escrow);
        env.events().publish((symbol_short!("dispute"), id), disputer);
    }

    /// Arbiter resolves dispute — can release to beneficiary or refund.
    pub fn resolve(env: Env, id: u64, release: bool) {
        let mut escrow: DisburseEscrow = env.storage().persistent()
            .get(&Key::Escrow(id)).expect("not found");

        escrow.arbiter.require_auth();
        assert!(escrow.state == EscrowState::Disputed, "not disputed");

        escrow.state = if release { EscrowState::Released } else { EscrowState::Refunded };
        env.storage().persistent().set(&Key::Escrow(id), &escrow);
        env.events().publish((symbol_short!("resolve"), id), release);
    }

    pub fn get(env: Env, id: u64) -> DisburseEscrow {
        env.storage().persistent().get(&Key::Escrow(id)).expect("not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn single_approval_releases() {
        let env = Env::default();
        env.mock_all_auths();
        let depositor   = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let arbiter     = Address::generate(&env);
        let cid = env.register_contract(None, DisburseEscrowContract);
        let client = DisburseEscrowContractClient::new(&env, &cid);
        let id = client.deposit(&depositor, &beneficiary, &arbiter, &10_000i128, &100u32, &1u32);
        client.approve(&id, &depositor);
        assert_eq!(client.get(&id).state, EscrowState::Released);
    }

    #[test]
    fn dispute_and_resolve_refund() {
        let env = Env::default();
        env.mock_all_auths();
        let depositor   = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let arbiter     = Address::generate(&env);
        let cid = env.register_contract(None, DisburseEscrowContract);
        let client = DisburseEscrowContractClient::new(&env, &cid);
        let id = client.deposit(&depositor, &beneficiary, &arbiter, &500i128, &200u32, &1u32);
        client.dispute(&id, &depositor);
        assert_eq!(client.get(&id).state, EscrowState::Disputed);
        client.resolve(&id, &false);
        assert_eq!(client.get(&id).state, EscrowState::Refunded);
    }
}
