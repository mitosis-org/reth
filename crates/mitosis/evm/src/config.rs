use crate::MitosisBlockExecutorFactory;
use reth_ethereum_primitives::EthPrimitives;
use alloy_consensus::Header;
use alloy_evm::EvmEnv;
use reth_chainspec::ChainSpec;
use reth_ethereum::{
    evm::{EthBlockAssembler, EthEvmConfig}, node::api::ConfigureEvm
};
use reth_ethereum_primitives::Block;
use reth_primitives_traits::{SealedBlock, SealedHeader};
use reth_evm::eth::EthBlockExecutionCtx;
use std::sync::Arc;

/// Mitosis EVM configuration that wraps the Ethereum EVM configuration
/// with custom block executor factory.
#[derive(Debug, Clone)]
pub struct MitosisEvmConfig {
    pub(super) inner: EthEvmConfig,
    pub(super) block_executor_factory: MitosisBlockExecutorFactory<<EthEvmConfig as ConfigureEvm>::BlockExecutorFactory>,
}

impl MitosisEvmConfig {
    /// Creates a new Mitosis EVM configuration with the given chain specification.
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        let inner = EthEvmConfig::new(chain_spec.clone());
        let block_executor_factory = MitosisBlockExecutorFactory::new(inner.block_executor_factory().clone());
        Self {
            inner,
            block_executor_factory,
        }
    }
}

impl ConfigureEvm for MitosisEvmConfig {
    type Primitives = EthPrimitives;
    type Error = <EthEvmConfig as ConfigureEvm>::Error;
    type NextBlockEnvCtx = <EthEvmConfig as ConfigureEvm>::NextBlockEnvCtx;
    type BlockExecutorFactory = MitosisBlockExecutorFactory<<EthEvmConfig as ConfigureEvm>::BlockExecutorFactory>;
    type BlockAssembler = EthBlockAssembler;

    fn block_executor_factory(&self) -> &Self::BlockExecutorFactory {
        &self.block_executor_factory
    }

    fn block_assembler(&self) -> &Self::BlockAssembler {
        &self.inner.block_assembler
    }

    fn evm_env(&self, header: &Header) -> EvmEnv {
        self.inner.evm_env(header)
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &Self::NextBlockEnvCtx,
    ) -> Result<EvmEnv, Self::Error> {
        self.inner.next_evm_env(parent, attributes)
    }

    fn context_for_block<'a>(&self, block: &'a SealedBlock<Block>) -> EthBlockExecutionCtx<'a> {
        self.inner.context_for_block(block)
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader<Header>,
        attributes: Self::NextBlockEnvCtx,
    ) -> EthBlockExecutionCtx<'_> {
        self.inner.context_for_next_block(parent, attributes)
    }
}
