//! Cohabitation bootstrap — wire NodeCore into a host process that
//! owns the persist Engine + the edge runtime.
//!
//! # CIRIS 3.0 in-process cohabitation
//!
//! The agent process constructs **one** persist `Engine` + **one**
//! edge `Edge`; node-core, lens-core, and the agent itself all share
//! them. Per CIRISNodeCore#4 node-core NEVER constructs its own
//! `Engine` or tokio runtime — it consumes injected handles. This
//! module is that injection point.
//!
//! # What "install" does
//!
//! [`install`] constructs the [`NodeCore`] service over an injected
//! [`NodeCoreService`] handle and registers all 8 federation-consensus
//! message handlers on the host's `Edge`. After it returns, inbound
//! messages of those `MessageType`s — addressed to the host's
//! federation signing key (edge is Reticulum-native: addressing IS
//! identity) — dispatch into node-core. That is what makes node-core
//! "addressable by signing key" and mergeable into the agent.
//!
//! [`install_from_dispatch`] is the substrate adapter: it accepts
//! persist v1.11.0's [`NodeCoreDispatch`] enum (returned by
//! `Engine::node_core_service()` / `PyEngine::node_core_service()`,
//! CIRISPersist#90) and routes each backend variant into [`install`].
//!
//! # PyO3 form
//!
//! The agent runtime is Python, so the production entry point is a
//! `#[pyfunction]` thin-wrapping [`install_from_dispatch`]. That
//! wrapper is pending CIRISEdge's v0.3.0 PyO3 surface — edge does not
//! yet expose its `Edge` to Python (`src/ffi/pyo3.rs` is a stub:
//! `m.add_class::<Edge>()` commented out). The Rust surface here is
//! complete and the wrapper drops in unchanged once edge ships it.

use std::sync::Arc;

use ciris_edge::{Edge, EdgeError};
use ciris_persist::NodeCoreDispatch;

use crate::service::NodeCore;
use crate::substrate::NodeCoreService;

/// Wire a [`NodeCoreService`] into a host edge runtime.
///
/// Constructs [`NodeCore`] over the injected `service` handle and
/// registers all 8 federation-consensus handlers on `edge`
/// (`ContributionSubmit`, `VoteCast`, `DeferralRequest`,
/// `DeferralResponse`, `ExpertiseAttestationPublish`,
/// `ModerationEventPublish`, `SlashingAttestationPublish`,
/// `ReconsiderationRequest`).
///
/// The host owns `edge`; node-core borrows it for the registration.
/// The constructed `NodeCore` is moved into the registered handler
/// closures (each holds an `Arc<NodeCore<E>>`), so it lives as long
/// as the handlers stay registered — the caller does not retain it.
pub async fn install<E>(service: Arc<E>, edge: &Edge) -> Result<(), EdgeError>
where
    E: NodeCoreService + 'static,
{
    Arc::new(NodeCore::new(service)).install_handlers(edge).await
}

/// Substrate adapter — wire node-core in from persist's
/// [`NodeCoreDispatch`] (`Engine::node_core_service()`, persist
/// v1.11.0 / CIRISPersist#90).
///
/// `NodeCoreService` uses RPITIT and is not object-safe, so persist
/// returns a per-backend dispatch enum rather than
/// `Arc<dyn NodeCoreService>`. Each variant carries a concrete
/// backend that implements the trait; both route into [`install`]
/// identically — the backend choice is the host's, transparent to
/// node-core.
///
/// Both `Postgres` and `Sqlite` variants are matched
/// unconditionally. At the cohabitation triple, edge v0.13.1 pins
/// persist with `features = ["sqlite"]`, so Cargo feature unification
/// forces persist's `Sqlite` variant active in every build that links
/// edge — gating node-core's match arm on a local `sqlite` feature
/// would silently de-cover an enum variant the consumer can produce.
/// Node-core's default cohabitation target remains Postgres (the
/// safety.ciris.ai shape, MISSION §7.3); the `sqlite` feature is
/// retained for explicit opt-in symmetry. NodeCore#6.
pub async fn install_from_dispatch(
    dispatch: NodeCoreDispatch,
    edge: &Edge,
) -> Result<(), EdgeError> {
    match dispatch {
        NodeCoreDispatch::Postgres(backend) => install(backend, edge).await,
        NodeCoreDispatch::Sqlite(backend) => install(backend, edge).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `install` is generic over `NodeCoreService`; compile-checking
    // that the bound resolves is the unit-level guarantee. Running it
    // needs a live `Edge` (transport + injected engine) — that's
    // integration-harness territory, not a node-core unit test, the
    // same call `install_handlers` already makes.
    fn _assert_install_is_generic_over_node_core_service<E>()
    where
        E: NodeCoreService + 'static,
    {
        let _ = install::<E>;
    }
}
