# Permutate

Permutate exists as both a library and application for permutating generic lists of lists as
well as individual lists using an original Rust-based algorithm which works with references.
If the data you are working with is not best-handled with references, this isn't for you.
It has been developed primarily for the goal of inclusion within the Rust implementation of
the GNU Parallel program, which provides the ability to permutate a list of input lists.

The source code documentation may be found on [Docs.rs](https://docs.rs/permutate/).

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

So how fast is it? On my i5-2410M laptop (Quad Core 2.3 GHz Sandybridge Mobile CPU), I average 2,140,000 string
reference permutations per second running Gentoo Linux with the performance governor. If I were to scale to all CPU
cores, I would achieve around 8 million permutations per second. Not bad for a laptop.

If you want to compare the performance of your processor/implementation in comparison, this is how I conducted my test:

```sh
for char in A B C D E F G H I J; do echo $char >> A; done
time target/release/permutate --benchmark -n -f A :::: A :::: A :::: A :::: A :::: A :::: A :::: A
```

This will generate 10,000,000 permutations and print the time that it took for the process to complete. Divide the
time completed by 10,000,000 and you will have permutations per second.
