pub const MANPAGE: &'static str = r#"NAME
    permutate - efficient command-line permutator written in Rust

SYNOPSIS
    permutate [-f | -h] [ARGS... MODE]...

DESCRIPTION
    Permutate is a command-line permutator written in Rust, originally designed for inclusion
    within the Rust implementation of Parallel. Following the UNIX philosophy, permutate has
    additionally been spun into both an application and library project to serve as a standalone
    application. The syntax for permutate is nearly identical to Parallel.

OPTIONS
    -b, --benchmark
        Performs a benchmark by permutation all possible values without printing.

    -f, --files
        The first list of inputs will be interpreted as files.

    -h, --files
        Prints this help information.

    -n, --no-delimiters
        Disable the spaced deliminters between elements.

MODES
    :::
        All following arguments will be interpreted as arguments.

    :::+
        All following arguments will be appended to the previous list.

    ::::
        All following arguments will be interpreted as files.

    ::::+
        All following arguments from files will be appended to the previous list.

"#;
