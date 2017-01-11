///! The purpose of this module is to supply supporting miscellanious traits for use throughout the project.
mod digits;
mod numtoa;

/// The `Digits` trait is used to get the number of digits within a number.
pub use self::digits::Digits;

/// The `NumToA` trait converts integers into their string representation,
/// but stores the results in a mutable stack-allocated byte slice.
pub use self::numtoa::NumToA;
