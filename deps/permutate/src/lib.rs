//! # Permutate
//!
//! Permutate exists as both a library and application for permutating generic lists of lists as
//! well as individual lists using an original Rust-based algorithm which works with references.
//! If the data you are working with is not best-handled with references, this isn't for you.
//! It has been developed primarily for the goal of inclusion within the Rust implementation of
//! the GNU Parallel program, which provides the ability to permutate a list of input lists.
//!
//! ## Examples
//!
//! These are a list of examples on how to use the library to manipulate string-based data.
//! The only thing we need to ensure is that our list of strings is in the `&[&[&str]]` format.
//!
//! ### An individual list
//!
//! ```rust
//! extern crate permutate;
//! use permutate::Permutator;
//! use std::io::{self, Write};
//!
//! fn main() {
//!     let stdout = io::stdout();
//!     let mut stdout = stdout.lock();
//!     let list: &[&str] = &["one", "two", "three", "four"];
//!     let list = [list];
//!     let permutator = Permutator::new(&list[..]);
//!
//!     // NOTE: print! macros are incredibly slow, so printing directly
//!     // to stdout is faster.
//!     for permutation in permutator {
//!         for element in permutation {
//!             let _ = stdout.write(element.as_bytes());
//!         }
//!         let _ = stdout.write(b"\n");
//!     }
//! }
//! ```
//!
//! ### An array of arrays: `&[&[&str]]`
//!
//! ```rust
//! extern crate permutate;
//! use permutate::Permutator;
//! use std::io::{self, Write};
//!
//! fn main() {
//!     let stdout = io::stdout();
//!     let mut stdout = stdout.lock();
//!     let lists = [
//!         &["one", "two", "three"][..],
//!         &["four", "five", "six"][..],
//!         &["seven", "eight", "nine"][..],
//!     ];
//!     let permutator = Permutator::new(&lists[..]);
//!
//!     // NOTE: print! macros are incredibly slow, so printing directly
//!     // to stdout is faster.
//!     for permutation in permutator {
//!         for element in permutation {
//!             let _ = stdout.write(element.as_bytes());
//!         }
//!         let _ = stdout.write(b"\n");
//!     }
//! }
//! ```
//!
//! ### A Vector of Vector of Strings: `Vec<Vec<String>>`
//!
//! This is the most complicated example to accomplish because you have to convert, essentially,
//! A vector of a vector of vectors into a slice of a slice of a slice, as the String type itself
//! is a vector of characters.
//!
//! ```rust
//! extern crate permutate;
//! use permutate::Permutator;
//! use std::io::{self, Write};
//!
//! fn main() {
//!     let stdout = io::stdout();
//!     let mut stdout = stdout.lock();
//!     let lists: Vec<Vec<String>> = vec![
//!         vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
//!         vec!["four".to_owned(), "five".to_owned(), "six".to_owned()],
//!         vec!["seven".to_owned(), "eight".to_owned(), "nine".to_owned()],
//!     ];
//!
//!     // Convert the `Vec<Vec<String>>` into a `Vec<Vec<&str>>`
//!     let tmp: Vec<Vec<&str>> = lists.iter()
//!         .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
//!         .collect();
//!
//!     // Convert the `Vec<Vec<&str>>` into a `Vec<&[&str]>`
//!     let vector_of_arrays: Vec<&[&str]> = tmp.iter()
//!         .map(AsRef::as_ref).collect();
//!
//!     // Pass the `Vec<&[&str]>` as an `&[&[&str]]`
//!     let permutator = Permutator::new(&vector_of_arrays[..]);
//!
//!     // NOTE: print! macros are incredibly slow, so printing directly
//!     // to stdout is faster.
//!     for permutation in permutator {
//!         for element in permutation {
//!             let _ = stdout.write(element.as_bytes());
//!         }
//!         let _ = stdout.write(b"\n");
//!     }
//! }
//! ```
//!

/// The `Permutator` contains the state of the iterator as well as the references to inputs
/// that are being permutated. The input should be provided as an array of an array of references.
pub struct Permutator<'a, T: 'a + ?Sized> {
    /// The counter is used to point to the next permutation sequence.
    counter:        Counter,
    /// Tracks how many times the `Permutator` has been used.
    curr_iteration: usize,
    /// The maximum number of permutations until all possible values have been computed.
    max_iterations: usize,
    /// The internal data that the permutator is permutating against.
    lists:          &'a [&'a [&'a T]],
    /// The total number of lists that is being permutated with.
    nlists:         usize,
    /// Whether the permutator is permutating against a single list, or multiple lists.
    single_list:    bool
}

impl<'a, T: 'a + ?Sized> Permutator<'a, T> {
    /// Initialize a new `Permutator` with the list of input lists to permutate with.
    /// The input may be provided as either multiple lists via an array of arrays, or a single
    /// list as an array within an array.
    pub fn new(lists: &'a [&'a [&'a T]]) -> Permutator<T> {
        let mut nlists  = lists.len();
        let single_list = nlists == 1;

        // The max counter values are calculated as the number of elements
        // in a slice, minus one to account for the zeroth value.
        let nvalues = if single_list {
            nlists = lists[0].len();
            (0..nlists).map(|_| nlists - 1).collect::<Vec<usize>>()
        } else {
            lists.iter().map(|list| list.len() - 1).collect::<Vec<usize>>()
        };

        let max_iters = nvalues.iter().map(|x| x + 1).product();

        Permutator {
            counter: Counter {
                counter: vec![0; nlists],
                max:     nvalues,
            },
            curr_iteration: 0,
            lists:          lists,
            max_iterations: max_iters,
            nlists:         nlists,
            single_list:    single_list
        }
    }

    /// Resets the internal state of the `Permutator` to allow you to start permutating again.
    pub fn reset(&mut self) {
        self.counter.reset();
        self.curr_iteration = 0;
    }
}

impl<'a, T: 'a + ?Sized> Iterator for Permutator<'a, T> {
    type Item = Vec<&'a T>;

    fn next(&mut self) -> Option<Vec<&'a T>> {
        // Without this check, the permutator would cycle forever and never return `None`
        // because my incrementing algorithim prohibits it.
        if self.curr_iteration == self.max_iterations {
            return None
        }

        self.curr_iteration += 1;

        // Generates the next permutation sequence using the current counter.
        let output = if self.single_list {
            self.counter.counter.iter()
                .map(|value| self.lists[0][*value])
                .collect::<Vec<&T>>()
        } else {
            self.counter.counter.iter().enumerate()
                .map(|(list, value)| self.lists[list][*value])
                .collect::<Vec<&T>>()
        };

        // Increment the counter to point towards the next set of values.
        self.counter.increment(&self.nlists - 1);

        // Return the collected permutation
        Some(output)
    }
}

/// Tracks the state of the indexes of each list.
struct Counter {
    /// The current state of the counter
    counter: Vec<usize>,
    /// The max possible values for each counter
    max:     Vec<usize>
}

impl Counter {
    fn increment(&mut self, nlists: usize) {
        // Check to see if the Nth value is on it's bounds
        if self.counter[nlists] == self.max[nlists] {
            // Recurse until nlist is zero.
            if nlists != 0 {
                self.counter[nlists] = 0;
                self.increment(nlists - 1);
            }
        } else {
            // Increment the Nth value's index by one.
            self.counter[nlists] += 1;
        }
    }

    fn reset(&mut self) {
        for value in self.counter.iter_mut() { *value = 0; }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    // Check to see if exactly 1,000,000 permutations were collected.
    fn test_million_permutations() {
        let inputs = [
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"][..]
        ];

        assert_eq!(1_000_000, Permutator::new(&inputs[..]).count())
    }

    #[test]
    // Verify that the permutations are generated with the correct values,
    // in the correct order.
    fn test_permutation_values() {
        let inputs = [&["1", "2", "3"][..], &["1", "2", "3"][..], &["1", "2", "3"][..]];
        let expected = [
            &["1", "1", "1"][..], &["1", "1", "2"][..], &["1", "1", "3"][..],
            &["1", "2", "1"][..], &["1", "2", "2"][..], &["1", "2", "3"][..],
            &["1", "3", "1"][..], &["1", "3", "2"][..], &["1", "3", "3"][..],
            &["2", "1", "1"][..], &["2", "1", "2"][..], &["2", "1", "3"][..],
            &["2", "2", "1"][..], &["2", "2", "2"][..], &["2", "2", "3"][..],
            &["2", "3", "1"][..], &["2", "3", "2"][..], &["2", "3", "3"][..],
            &["3", "1", "1"][..], &["3", "1", "2"][..], &["3", "1", "3"][..],
            &["3", "2", "1"][..], &["3", "2", "2"][..], &["3", "2", "3"][..],
            &["3", "3", "1"][..], &["3", "3", "2"][..], &["3", "3", "3"][..],
        ];

        for (output, expected) in Permutator::new(&inputs[..]).zip(expected[..].iter()) {
            assert_eq!(&output, expected);
        }
    }

    #[test]
    fn single_list_permutation() {
        let input = [&["1", "2", "3"][..]];
        let expected = [
            &["1", "1", "1"][..], &["1", "1", "2"][..], &["1", "1", "3"][..],
            &["1", "2", "1"][..], &["1", "2", "2"][..], &["1", "2", "3"][..],
            &["1", "3", "1"][..], &["1", "3", "2"][..], &["1", "3", "3"][..],
            &["2", "1", "1"][..], &["2", "1", "2"][..], &["2", "1", "3"][..],
            &["2", "2", "1"][..], &["2", "2", "2"][..], &["2", "2", "3"][..],
            &["2", "3", "1"][..], &["2", "3", "2"][..], &["2", "3", "3"][..],
            &["3", "1", "1"][..], &["3", "1", "2"][..], &["3", "1", "3"][..],
            &["3", "2", "1"][..], &["3", "2", "2"][..], &["3", "2", "3"][..],
            &["3", "3", "1"][..], &["3", "3", "2"][..], &["3", "3", "3"][..],
        ];
        for (output, expected) in Permutator::new(&input[..]).zip(expected[..].iter()) {
            assert_eq!(&output, expected);
        }
    }

    #[test]
    fn test_reset() {
        let input = [&["1", "2", "3"][..]];
        let expected = [
            &["1", "1", "1"][..], &["1", "1", "2"][..], &["1", "1", "3"][..],
            &["1", "2", "1"][..], &["1", "2", "2"][..], &["1", "2", "3"][..],
            &["1", "3", "1"][..], &["1", "3", "2"][..], &["1", "3", "3"][..],
            &["2", "1", "1"][..], &["2", "1", "2"][..], &["2", "1", "3"][..],
            &["2", "2", "1"][..], &["2", "2", "2"][..], &["2", "2", "3"][..],
            &["2", "3", "1"][..], &["2", "3", "2"][..], &["2", "3", "3"][..],
            &["3", "1", "1"][..], &["3", "1", "2"][..], &["3", "1", "3"][..],
            &["3", "2", "1"][..], &["3", "2", "2"][..], &["3", "2", "3"][..],
            &["3", "3", "1"][..], &["3", "3", "2"][..], &["3", "3", "3"][..],
        ];
        let mut permutator = Permutator::new(&input[..]);
        for (output, expected) in permutator.by_ref().zip(expected[..].iter()) {
            assert_eq!(&output, expected);
        }
        permutator.reset();
        for (output, expected) in permutator.zip(expected[..].iter()) {
            assert_eq!(&output, expected);
        }
    }
}
