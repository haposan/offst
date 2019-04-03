#![crate_type = "lib"]
#![feature(futures_api, async_await, await_macro, arbitrary_self_types)]
#![feature(nll)]
#![feature(generators)]
#![feature(never_type)]
#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]
#![deny(trivial_numeric_casts, warnings)]
#![allow(intra_doc_link_resolution_failure)]

extern crate futures_cpupool;

#[macro_use]
extern crate log;

extern crate num_bigint;
extern crate num_traits;

extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate common;

mod credit_calc;
mod ephemeral;
mod friend;
mod funder;
mod handler;
mod liveness;
mod mutual_credit;
pub mod report;
mod state;
#[cfg(test)]
mod tests;
mod token_channel;
pub mod types;

pub use self::funder::{funder_loop, FunderError};
pub use self::state::{FunderMutation, FunderState};
