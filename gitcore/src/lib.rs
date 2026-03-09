//! gitcore — CRUST VCS core library
//!
//! This library provides pure Rust implementation of the CRUST object model.
//! No async, no network, no database, no HTTP.
//!
//! It can be tested with `cargo test -p gitcore` with no external services.

#![warn(missing_docs)]

pub mod blob;
pub mod commit;
pub mod error;
pub mod merge;
pub mod object;
pub mod tag;
pub mod tree;

pub use blob::Blob;
pub use commit::Commit;
pub use error::{Error, Result};
pub use object::{Object, ObjectId, ObjectType};
pub use tag::Tag;
pub use tree::{Tree, TreeEntry};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_loads() {
        assert_eq!(crate::VERSION, "0.1.0");
    }
}
