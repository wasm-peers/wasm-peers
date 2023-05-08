/*!
This crate provides an easy-to-use wrapper around `WebRTC` and `DataChannels` for a peer to peer connections.

# Overview

As creator of [agar.io](https://agar.io) famously stated [`WebRTC` is hard](https://news.ycombinator.com/item?id=13264952).
This library aims to help, by abstracting away all the setup, and providing a simple way to send
and receive messages over the data channel.

It's as easy as providing address to a signaling server instance from
[accompanying crate](https://docs.rs/wasm-peers-signaling-server/latest/wasm_peers_signaling_server/) and specifying two callbacks.
One for when a connection opens, and one for when a message is received.
After that you can send messages back and forth without worrying about the implementation details.

Library contains three network , [one-to-one](one_to_one), which creates an equal connection between two peers,
[one-to-many](one_to_many), which specifies a host and arbitrary number of clients
and [many-to-many] that creates connection for each pair of peers and allows sending messages to any of them.

*/

#![allow(
    clippy::module_name_repetitions,
    clippy::future_not_send, // false positive in WASM (single threaded) context
)]
// clippy WARN level lints
#![warn(
    // missing_docs,
    clippy::cargo,
    clippy::pedantic,
    // clippy::nursery,
    clippy::dbg_macro,
    clippy::unwrap_used,
    clippy::integer_division,
    clippy::large_include_file,
    clippy::map_err_ignore,
    // clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::todo,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unreachable
)]
// clippy WARN level lints, that can be upgraded to DENY if preferred
#![warn(
    clippy::float_arithmetic,
    clippy::integer_arithmetic,
    clippy::modulo_arithmetic,
    clippy::as_conversions,
    clippy::assertions_on_result_states,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::default_union_representation,
    clippy::deref_by_slicing,
    clippy::empty_drop,
    clippy::empty_structs_with_brackets,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::pattern_type_mismatch,
    clippy::string_slice,
    clippy::try_err
)]
// clippy DENY level lints, they always have a quick fix that should be preferred
#![deny(
    clippy::wildcard_imports,
    clippy::multiple_inherent_impl,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::self_named_module_files,
    clippy::separated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_to_string,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::unseparated_literal_suffix,
    clippy::verbose_file_reads
)]

pub(crate) mod constants;
mod error;
#[cfg(feature = "many-to-many")]
pub mod many_to_many;
#[cfg(feature = "one-to-many")]
pub mod one_to_many;
#[cfg(feature = "one-to-one")]
pub mod one_to_one;
mod utils;

pub use error::{Error, Result};
pub use utils::{get_random_session_id, ConnectionType};
pub use wasm_peers_protocol::{SessionId, UserId};
