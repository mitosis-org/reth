//! Mitosis-specific system calls for contract deployment.

use alloy_evm::{
    block::BlockExecutionError,
    Evm,
};
use alloy_primitives::{keccak256, U256};
use revm::{
    state::{Account, AccountInfo, AccountStatus, Bytecode, EvmState},
    Database, DatabaseCommit,
};
use reth_mitosis_primitives::{
    MULTICALL3_REPLACEMENT_BLOCK, MULTICALL3_HARDFORK_CHAIN_ID, MULTICALL3_ADDRESS, get_multicall3_bytecode
};

/// Deploys the Multicall3 contract at the specific chain id and block number by directly modifying the state.
///
/// This function properly loads the account into the cache first, then modifies it
/// to ensure REVM's cache consistency requirements are met.
#[inline]
pub fn deploy_multicall3_contract<Halt, E>(
    evm: &mut E,
) -> Result<(), BlockExecutionError>
where
    E: Evm<HaltReason = Halt> + ?Sized,
    E::DB: Database + DatabaseCommit,
{
    // Only deploy at the specific chain id
    // TODO: This is a temporary solution to ensure the Multicall3 contract is deployed at the correct chain id
    // We should implement a correct hardfork logic
    let chain_id = evm.chain_id();
    if chain_id != MULTICALL3_HARDFORK_CHAIN_ID {
        return Ok(());
    }

    // Only deploy at the specific block
    let block_number = evm.block().number.saturating_to::<u64>();
    if block_number != MULTICALL3_REPLACEMENT_BLOCK {
        return Ok(());
    }

    // First, load the account into cache by calling basic() on the database
    // This ensures the account is present in the cache before we modify it
    let _existing_account = evm.db_mut().basic(MULTICALL3_ADDRESS)
        .map_err(|_| BlockExecutionError::Internal(
            alloy_evm::block::InternalBlockExecutionError::Other(
                "Failed to load account into cache".into()
            )
        ))?;

    // Get the Multicall3 bytecode
    let bytecode = get_multicall3_bytecode();
    
    // Create the account info with the bytecode
    let account_info = AccountInfo {
        balance: _existing_account.map(|acc| acc.balance).unwrap_or(U256::ZERO),
        nonce: 1, // Contract nonce (1 indicates it's a contract)
        code_hash: keccak256(&bytecode),
        code: Some(Bytecode::new_raw(bytecode)),
    };
    
    // Create the account with the proper status
    let account = Account {
        info: account_info,
        transaction_id: 0, // System-level change, no specific transaction
        storage: Default::default(),
        status: AccountStatus::Touched | AccountStatus::Created,
    };
    
    // Create state changes
    let mut state = EvmState::default();
    state.insert(MULTICALL3_ADDRESS, account);
    
    // Commit the state changes directly
    evm.db_mut().commit(state);
    
    Ok(())
}

/// Apply Multicall3 deployment during pre-execution phase.
///
/// This should be called in the `apply_pre_execution_changes` method of the block executor.
pub fn apply_multicall3_deployment<Halt, E>(
    evm: &mut E,
) -> Result<(), BlockExecutionError>
where
    E: Evm<HaltReason = Halt> + ?Sized,
    E::DB: Database + DatabaseCommit,
{
    // Deploy Multicall3 directly to state
    deploy_multicall3_contract(evm)
}
