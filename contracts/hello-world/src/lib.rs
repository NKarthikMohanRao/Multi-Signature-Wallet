#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Address, Vec, symbol_short, Symbol};

// Structure to store transaction proposal details
#[contracttype]
#[derive(Clone)]
pub struct Transaction {
    pub tx_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub approvals: u32,
    pub executed: bool,
}

// Structure to store wallet configuration
#[contracttype]
#[derive(Clone)]
pub struct WalletConfig {
    pub owners: Vec<Address>,
    pub required_approvals: u32,
    pub total_transactions: u64,
}

// Storage keys
const WALLET_CONFIG: Symbol = symbol_short!("W_CONFIG");
const TX_COUNT: Symbol = symbol_short!("TX_COUNT");

#[contracttype]
pub enum TransactionBook {
    Transaction(u64)
}

#[contracttype]
pub enum ApprovalBook {
    Approval(u64, Address) // (tx_id, owner_address)
}

#[contract]
pub struct MultiSigWallet;

#[contractimpl]
impl MultiSigWallet {
    
    // Initialize the wallet with owners and required approvals
    pub fn initialize(env: Env, owners: Vec<Address>, required_approvals: u32) {
        
        // Validate inputs
        if owners.len() == 0 {
            panic!("Must have at least one owner");
        }
        
        if required_approvals == 0 || required_approvals > owners.len() {
            panic!("Invalid number of required approvals");
        }
        
        let config = WalletConfig {
            owners: owners.clone(),
            required_approvals,
            total_transactions: 0,
        };
        
        env.storage().instance().set(&WALLET_CONFIG, &config);
        env.storage().instance().set(&TX_COUNT, &0u64);
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Multi-Sig Wallet initialized with {} owners and {} required approvals", 
             owners.len(), required_approvals);
    }
    
    // Propose a new transaction
    pub fn propose_transaction(env: Env, proposer: Address, recipient: Address, amount: i128) -> u64 {
        proposer.require_auth();
        
        let config = Self::get_wallet_config(env.clone());
        
        // Verify proposer is an owner
        if !Self::is_owner(env.clone(), proposer.clone()) {
            panic!("Only owners can propose transactions");
        }
        
        let mut tx_count: u64 = env.storage().instance().get(&TX_COUNT).unwrap_or(0);
        tx_count += 1;
        
        let transaction = Transaction {
            tx_id: tx_count,
            recipient: recipient.clone(),
            amount,
            approvals: 0,
            executed: false,
        };
        
        env.storage().instance().set(&TransactionBook::Transaction(tx_count), &transaction);
        env.storage().instance().set(&TX_COUNT, &tx_count);
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Transaction {} proposed by owner to send {} to recipient", tx_count, amount);
        tx_count
    }
    
    // Approve a transaction
    pub fn approve_transaction(env: Env, approver: Address, tx_id: u64) {
        approver.require_auth();
        
        // Verify approver is an owner
        if !Self::is_owner(env.clone(), approver.clone()) {
            panic!("Only owners can approve transactions");
        }
        
        let mut transaction = Self::get_transaction(env.clone(), tx_id);
        
        if transaction.executed {
            panic!("Transaction already executed");
        }
        
        // Check if already approved by this owner
        let approval_key = ApprovalBook::Approval(tx_id, approver.clone());
        if env.storage().instance().has(&approval_key) {
            panic!("Owner has already approved this transaction");
        }
        
        // Record approval
        env.storage().instance().set(&approval_key, &true);
        transaction.approvals += 1;
        
        env.storage().instance().set(&TransactionBook::Transaction(tx_id), &transaction);
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Transaction {} approved. Current approvals: {}", tx_id, transaction.approvals);
    }
    
    // Execute a transaction if it has enough approvals
    pub fn execute_transaction(env: Env, executor: Address, tx_id: u64) {
        executor.require_auth();
        
        // Verify executor is an owner
        if !Self::is_owner(env.clone(), executor.clone()) {
            panic!("Only owners can execute transactions");
        }
        
        let mut transaction = Self::get_transaction(env.clone(), tx_id);
        let config = Self::get_wallet_config(env.clone());
        
        if transaction.executed {
            panic!("Transaction already executed");
        }
        
        if transaction.approvals < config.required_approvals {
            panic!("Not enough approvals to execute transaction");
        }
        
        // Mark as executed
        transaction.executed = true;
        env.storage().instance().set(&TransactionBook::Transaction(tx_id), &transaction);
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Transaction {} executed: {} sent to recipient", tx_id, transaction.amount);
    }
    
    // Helper function to check if an address is an owner
    fn is_owner(env: Env, address: Address) -> bool {
        let config = Self::get_wallet_config(env.clone());
        
        for i in 0..config.owners.len() {
            if config.owners.get(i).unwrap() == address {
                return true;
            }
        }
        false
    }
    
    // Helper function to get wallet configuration
    fn get_wallet_config(env: Env) -> WalletConfig {
        env.storage().instance().get(&WALLET_CONFIG).unwrap_or_else(|| {
            panic!("Wallet not initialized");
        })
    }
    
    // Helper function to get transaction details
    fn get_transaction(env: Env, tx_id: u64) -> Transaction {
        env.storage().instance().get(&TransactionBook::Transaction(tx_id))
            .unwrap_or_else(|| {
                panic!("Transaction not found");
            })
    }
}