#![feature(futures_api, async_await, await_macro, arbitrary_self_types)]
#![feature(nll)]
#![feature(try_from)]
#![feature(generators)]
#![feature(never_type)]

#![deny(
    trivial_numeric_casts,
    warnings
)]

#![allow(unused)]

#[macro_use]
extern crate log;

mod setup_conn;
pub mod identity;
mod connect;
mod node_connection;