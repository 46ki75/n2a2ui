//! Rust types for the A2UI v0.9 Elmethis Block Catalog.
//!
//! A2UI is a declarative, streaming JSON protocol that lets agents describe
//! UI structure as a flat adjacency list of components, which the client
//! renders with its own native widgets. This crate models the v0.9
//! Elmethis Block Catalog as Rust structs that round-trip through serde.

pub mod v0_9;
