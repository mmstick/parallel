# Permutate

Permutate exists as both a library and application for permutating generic lists of lists as
well as individual lists using an original Rust-based algorithm which works with references.
If the data you are working with is not best-handled with references, this isn't for you.
It has been developed primarily for the goal of inclusion within the Rust implementation of
the GNU Parallel program, which provides the ability to permutate a list of input lists.

The source code documentation may be found on [Docs.rs](https://docs.rs/permutate/0.1.0/permutate/).

## Application

Following the spirit of the Rust and UNIX philosophy, I am also releasing this as it's own simple application to
bring the capabilities of the permutate to the command-line, because shell lives matter. The syntax is very much
identical to GNU Parallel, so users of GNU Parallel will be right at home with this command.

```sh
$ permutate A B ::: C D ::: E F
A C E
A C F
A D E
A D F
B C E
B C F
B D E
B D F
```

```sh
$ permutate -n A B ::: C D ::: E F
ACE
ACF
ADE
ADF
BCE
BCF
BDE
BDF
```

Other accepted syntaxes are:

```sh
$ permutate -f file file :::+ arg arg :::: file file ::::+ file file ::: arg arg

```

### Benchmark

So how fast is it? On my i5-2410M laptop (Quad Core 2.3 GHz Sandybridge Mobile CPU), I average 19,400,000 string
reference permutations per second running Gentoo Linux with the performance governor. If I were to scale to all CPU
cores, I would achieve around 80 million permutations per second. Not bad for a laptop.

If you want to compare the performance of your processor/implementation in comparison, this is how I conducted my test:

```sh
for char in A B C D E F G H I J; do echo $char >> A; done
time target/release/permutate --benchmark -f A :::: A :::: A :::: A :::: A :::: A :::: A :::: A
```

This will generate 100,000,000 permutations and print the time that it took for the process to complete. Divide the
time completed by 100,000,000 and you will have permutations per second.

## Library

### Examples

These are a list of examples on how to use the library to manipulate string-based data.
The only thing we need to ensure is that our list of strings is in the `&[&[str]]` format.

#### An individual list

```rust
extern crate permutate;
use permutate::Permutator;
use std::io::{self, Write};

fn main() {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let list: &[&str]   = &["one", "two", "three", "four"];
    let list: [&[&str]] = [list];
    let permutator = Permutator::new(&list[..]);

    // NOTE: print! macros are incredibly slow, so printing directly
    // to stdout is faster.
    for permutation in permutator {
        for element in permutation {
            let _ = stdout.write(element.as_bytes());
        }
        let _ = stdout.write(b"\n");
    }
}
```

#### An array of arrays: `&[&[&str]]`

```rust
extern crate permutate;
use permutate::Permutator;
use std::io::{self, Write};

fn main() {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let lists = [
        &["one", "two", "three"][..],
        &["four", "five", "six"][..],
        &["seven", "eight", "nine"][..],
    ];
    let permutator = Permutator::new(&lists[..]);

    // NOTE: print! macros are incredibly slow, so printing directly
    // to stdout is faster.
    for permutation in permutator {
        for element in permutation {
            let _ = stdout.write(element.as_bytes());
        }
        let _ = stdout.write(b"\n");
    }
}
```

#### A Vector of Vector of Strings: `Vec<Vec<String>>`

This is the most complicated example to accomplish because you have to convert, essentially,
A vector of a vector of vectors into a slice of a slice of a slice, as the String type itself
is a vector of characters.

```rust
extern crate permutate;
use permutate::Permutator;
use std::io::{self, Write};

fn main() {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let lists: Vec<Vec<String>> = vec![
        vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
        vec!["four".to_owned(), "five".to_owned(), "six".to_owned()],
        vec!["seven".to_owned(), "eight".to_owned(), "nine".to_owned()],
    ];

    // Convert the `Vec<Vec<String>>` into a `Vec<Vec<&str>>`
    let tmp: Vec<Vec<&str>> = lists.iter()
        .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
        .collect();

    // Convert the `Vec<Vec<&str>>` into a `Vec<&[&str]>`
    let vector_of_arrays: Vec<&[&str]> = tmp.iter()
        .map(AsRef::as_ref).collect();

    // Pass the `Vec<&[&str]>` as an `&[&[&str]]`
    let permutator = Permutator::new(&vector_of_arrays[..]);

    // NOTE: print! macros are incredibly slow, so printing directly
    // to stdout is faster.
    for permutation in permutator {
        for element in permutation {
            let _ = stdout.write(element.as_bytes());
        }
        let _ = stdout.write(b"\n");
    }
}
```
