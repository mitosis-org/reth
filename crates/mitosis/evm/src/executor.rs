use alloy_evm::{
    block::{
        BlockExecutionError, BlockExecutionResult, BlockExecutor,
        BlockExecutorFactory, BlockExecutorFor, CommitChanges, 
        ExecutableTx, OnStateHook,
    },
    Database, Evm, EvmFactory,
};
use alloy_primitives::Log;
use alloy_consensus::{Transaction, TxReceipt};
use alloy_eips::eip2718::Encodable2718;
use revm::{context::result::ExecutionResult, Inspector, DatabaseCommit};
use crate::system_calls::apply_multicall3_deployment;

/// Wrapper block executor for Mitosis that delegates to an inner executor
/// while maintaining full compatibility with existing functionality.
#[derive(Debug)]
pub struct MitosisBlockExecutor<Inner> {
    inner: Inner,
}

impl<Inner> MitosisBlockExecutor<Inner> {
    /// Creates a new MitosisBlockExecutor wrapping the given executor.
    pub fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> BlockExecutor for MitosisBlockExecutor<Inner>
where
    Inner: BlockExecutor,
    Inner::Transaction: Transaction + Encodable2718,
    Inner::Receipt: TxReceipt<Log = Log>,
    <Inner::Evm as Evm>::DB: Database + DatabaseCommit,
{
    type Transaction = Inner::Transaction;
    type Receipt = Inner::Receipt;
    type Evm = Inner::Evm;

    fn apply_pre_execution_changes(&mut self) -> Result<(), BlockExecutionError> {
        // First apply the inner pre-execution changes

        self.inner.apply_pre_execution_changes()?;
        
        // Deploy Multicall3 contract using system transaction at block 150
        apply_multicall3_deployment(self.inner.evm_mut())?;
        
        Ok(())
    }

    fn execute_transaction_with_commit_condition(
        &mut self,
        tx: impl ExecutableTx<Self>,
        f: impl FnOnce(&ExecutionResult<<Self::Evm as Evm>::HaltReason>) -> CommitChanges,
    ) -> Result<Option<u64>, BlockExecutionError> {
        self.inner.execute_transaction_with_commit_condition(tx, f)
    }

    fn finish(
        self,
    ) -> Result<(Self::Evm, BlockExecutionResult<Self::Receipt>), BlockExecutionError> {
        // Let the inner executor finish
        // The Multicall3 deployment is handled at a different layer
        self.inner.finish()
    }

    fn set_state_hook(&mut self, hook: Option<Box<dyn OnStateHook>>) {
        self.inner.set_state_hook(hook)
    }

    fn evm_mut(&mut self) -> &mut Self::Evm {
        self.inner.evm_mut()
    }

    fn evm(&self) -> &Self::Evm {
        self.inner.evm()
    }
}

/// Factory for creating MitosisBlockExecutor instances.
#[derive(Debug, Clone)]
pub struct MitosisBlockExecutorFactory<Inner> {
    inner: Inner,
}

impl<Inner> MitosisBlockExecutorFactory<Inner> {
    /// Creates a new MitosisBlockExecutorFactory.
    pub const fn new(inner: Inner) -> Self {
        Self { inner }
    }
}

impl<Inner> BlockExecutorFactory for MitosisBlockExecutorFactory<Inner>
where
    Inner: BlockExecutorFactory,
    Inner::Transaction: Transaction + Encodable2718,
    Inner::Receipt: TxReceipt<Log = Log>,
    Self: 'static,
{
    type EvmFactory = Inner::EvmFactory;
    type ExecutionCtx<'a> = Inner::ExecutionCtx<'a>;
    type Transaction = Inner::Transaction;
    type Receipt = Inner::Receipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        self.inner.evm_factory()
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: <Self::EvmFactory as EvmFactory>::Evm<&'a mut revm::database::State<DB>, I>,
        ctx: Self::ExecutionCtx<'a>,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + 'a,
        I: Inspector<<Self::EvmFactory as EvmFactory>::Context<&'a mut revm::database::State<DB>>> + 'a,
    {
        MitosisBlockExecutor::new(self.inner.create_executor(evm, ctx))
    }
}
