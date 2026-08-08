//! Stigmergic Agent Coordination (F9) — ant-colony-inspired signals.
//!
//! Agents leave pheromone-like signals on files and symbols, enabling
//! implicit coordination between multiple agents in the same codebase.
#![allow(unreachable_pub)]

mod pressure;
mod signal;

#[allow(unused_imports)]
pub(crate) use pressure::{PressureField, PressureMap};
pub use signal::PheromoneSignal;
#[allow(unused_imports)]
pub(crate) use signal::{deposit_signal, evaporate, read_signals};
