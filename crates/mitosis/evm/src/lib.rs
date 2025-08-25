//! Mitosis EVM implementation
//!
//! This crate provides EVM configuration and execution components specifically
//! tailored for the Mitosis network, including custom precompiles and execution logic.

#![warn(missing_docs)]

mod config;
mod executor;
mod system_calls;

pub use config::MitosisEvmConfig;
pub use executor::{MitosisBlockExecutor, MitosisBlockExecutorFactory};
pub use system_calls::{apply_multicall3_deployment, deploy_multicall3_contract};

use alloy_evm::{eth::EthEvmContext, precompiles::PrecompilesMap, EvmFactory};

use reth_ethereum::{
    evm::{
        primitives::{Database, EvmEnv},
        revm::{
            context::{Context, TxEnv},
            context_interface::result::{EVMError, HaltReason},
            handler::EthPrecompiles,
            inspector::{Inspector, NoOpInspector},
            interpreter::interpreter::EthInterpreter,
            primitives::hardfork::SpecId,
            MainContext, MainBuilder,
        },
        EthEvm,
    },
    node::{
        api::{FullNodeTypes, NodeTypes},
        builder::{components::ExecutorBuilder, BuilderContext},
    },
    EthPrimitives,
};
use reth_chainspec::ChainSpec;

/// Mitosis EVM configuration factory.
///
/// This factory creates EVM instances with Mitosis-specific customizations,
/// including custom precompiles and contract replacements.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MitosisEvmFactory;

impl EvmFactory for MitosisEvmFactory {
    type Evm<DB: Database, I: Inspector<EthEvmContext<DB>, EthInterpreter>> =
        EthEvm<DB, I, Self::Precompiles>;
    type Tx = TxEnv;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;
    type Context<DB: Database> = EthEvmContext<DB>;
    type Spec = SpecId;
    type Precompiles = PrecompilesMap;

    fn create_evm<DB: Database>(&self, db: DB, input: EvmEnv) -> Self::Evm<DB, NoOpInspector> {
        let evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {})
            .with_precompiles(PrecompilesMap::from_static(EthPrecompiles::default().precompiles));

        EthEvm::new(evm, false)
    }

    fn create_evm_with_inspector<DB: Database, I: Inspector<Self::Context<DB>, EthInterpreter>>(
        &self,
        db: DB,
        input: EvmEnv,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        EthEvm::new(self.create_evm(db, input).into_inner().with_inspector(inspector), true)
    }
}

/// Mitosis executor builder that uses the custom EVM factory.
///
/// This builder creates block executors configured with Mitosis-specific
/// EVM settings and customizations.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct MitosisExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for MitosisExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type EVM = MitosisEvmConfig;

    async fn build_evm(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::EVM> {
        let evm_config = MitosisEvmConfig::new(ctx.chain_spec());
        Ok(evm_config)
    }
}
