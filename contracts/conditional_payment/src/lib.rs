#![no_std]
//! ConditionalPayment — holds funds on-chain and releases them when an
//! authorised oracle confirms a condition (delivery, KYC approval, etc.).
//! Used by the StellarDisburse SDK for aid disbursement and trade finance flows.
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum PaymentStatus {
    Pending,
    Released,
    Refunded,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct ConditionalPayment {
    pub payer: Address,
    pub payee: Address,
    pub oracle: Address,          // authorised condition confirmer
    pub amount: i128,
    pub condition_id: String,     // external reference (invoice ID, KYC ref, etc.)
    pub timeout_ledger: u32,
    pub status: PaymentStatus,
}

#[contracttype]
pub enum Key {
    Payment(u64),
    Counter,
}

#[contract]
pub struct ConditionalPaymentContract;

#[contractimpl]
impl ConditionalPaymentContract {
    /// Deposit funds. Payer must authorise.
    pub fn deposit(
        env: Env,
        payer: Address,
        payee: Address,
        oracle: Address,
        amount: i128,
        condition_id: String,
        timeout_ledgers: u32,
    ) -> u64 {
        payer.require_auth();
        assert!(amount > 0, "amount must be positive");
        assert!(timeout_ledgers > 0, "timeout must be positive");

        let id: u64 = env.storage().instance()
            .get(&Key::Counter).unwrap_or(0u64) + 1;
        env.storage().instance().set(&Key::Counter, &id);

        let payment = ConditionalPayment {
            payer,
            payee,
            oracle,
            amount,
            condition_id: condition_id.clone(),
            timeout_ledger: env.ledger().sequence() + timeout_ledgers,
            status: PaymentStatus::Pending,
        };

        env.storage().persistent().set(&Key::Payment(id), &payment);
        env.events().publish(
            (symbol_short!("deposit"), id),
            (amount, condition_id),
        );
        id
    }

    /// Oracle confirms the condition — releases funds to payee.
    pub fn confirm(env: Env, id: u64) {
        let mut payment: ConditionalPayment = env.storage().persistent()
            .get(&Key::Payment(id)).expect("payment not found");

        payment.oracle.require_auth();
        assert!(payment.status == PaymentStatus::Pending, "not pending");
        assert!(
            env.ledger().sequence() < payment.timeout_ledger,
            "payment has timed out"
        );

        payment.status = PaymentStatus::Released;
        env.storage().persistent().set(&Key::Payment(id), &payment);
        env.events().publish((symbol_short!("release"), id), payment.amount);
    }

    /// Refund to payer after timeout. Anyone can trigger after timeout.
    pub fn timeout_refund(env: Env, id: u64) {
        let mut payment: ConditionalPayment = env.storage().persistent()
            .get(&Key::Payment(id)).expect("payment not found");

        assert!(payment.status == PaymentStatus::Pending, "not pending");
        assert!(
            env.ledger().sequence() >= payment.timeout_ledger,
            "timeout not reached yet"
        );

        payment.status = PaymentStatus::Refunded;
        env.storage().persistent().set(&Key::Payment(id), &payment);
        env.events().publish((symbol_short!("refund"), id), payment.amount);
    }

    /// Cancel before timeout. Only payer can cancel.
    pub fn cancel(env: Env, id: u64) {
        let mut payment: ConditionalPayment = env.storage().persistent()
            .get(&Key::Payment(id)).expect("payment not found");

        payment.payer.require_auth();
        assert!(payment.status == PaymentStatus::Pending, "not pending");

        payment.status = PaymentStatus::Cancelled;
        env.storage().persistent().set(&Key::Payment(id), &payment);
        env.events().publish((symbol_short!("cancel"), id), ());
    }

    pub fn get(env: Env, id: u64) -> ConditionalPayment {
        env.storage().persistent().get(&Key::Payment(id)).expect("not found")
    }

    pub fn status(env: Env, id: u64) -> PaymentStatus {
        let p: ConditionalPayment = env.storage().persistent()
            .get(&Key::Payment(id)).expect("not found");
        p.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let payer  = Address::generate(&env);
        let payee  = Address::generate(&env);
        let oracle = Address::generate(&env);
        (env, payer, payee, oracle)
    }

    #[test]
    fn deposit_and_confirm() {
        let (env, payer, payee, oracle) = setup();
        let cid = env.register_contract(None, ConditionalPaymentContract);
        let client = ConditionalPaymentContractClient::new(&env, &cid);
        let cond = String::from_str(&env, "INV-2026-001");
        let id = client.deposit(&payer, &payee, &oracle, &5_000i128, &cond, &100u32);
        assert_eq!(client.status(&id), PaymentStatus::Pending);
        client.confirm(&id);
        assert_eq!(client.status(&id), PaymentStatus::Released);
    }

    #[test]
    fn cancel_by_payer() {
        let (env, payer, payee, oracle) = setup();
        let cid = env.register_contract(None, ConditionalPaymentContract);
        let client = ConditionalPaymentContractClient::new(&env, &cid);
        let cond = String::from_str(&env, "INV-2026-002");
        let id = client.deposit(&payer, &payee, &oracle, &1_000i128, &cond, &200u32);
        client.cancel(&id);
        assert_eq!(client.status(&id), PaymentStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "not pending")]
    fn double_confirm_panics() {
        let (env, payer, payee, oracle) = setup();
        let cid = env.register_contract(None, ConditionalPaymentContract);
        let client = ConditionalPaymentContractClient::new(&env, &cid);
        let cond = String::from_str(&env, "INV-2026-003");
        let id = client.deposit(&payer, &payee, &oracle, &500i128, &cond, &100u32);
        client.confirm(&id);
        client.confirm(&id); // should panic
    }
}
