pub const MAN_PAGE: &'static str = r#"NAME
    parallel - a command-line CPU load balancer written in Rust

SYNOPSIS
    parallel [OPTIONS...] 'COMMAND' ::: INPUTS...
    parallel [OPTIONS...] ::: 'COMMANDS'...
    COMMAND | parallel [OPTIONS...] ::: INPUTS...

DESCRIPTION
    Parallel parallelizes otherwise non-parallel command-line tasks. When
    there are a number of commands that need to be executed, which may be
    executed in parallel, the Parallel application will evenly distribute
    tasks to all available CPU cores. There are three basic methods for how
    commands are supplied:

    1. A COMMAND may be defined, followed by a ::: separator which denotes
       that all following arguments will be usde as INPUTS for the command.

    2. If no COMMAND is provided, then the INPUTS will be interpreted as
       COMMANDS.

    3. If no INPUTS are provided, then standard input will be read for INPUTS.

    By default, Parallel groups the standard output and error of each child
    process so that outputs are printed in the order that they are given, as
    if the tasks were executed serially in a traditional for loop. In addition,
    commands are executed in the platform's preferred shell by default, which
    is `sh -c` on Unix systems, and `cmd /C` on Windows. These both come at a
    performance cost, so they can be disabled with the --ungroup and --no-shell
    options.

INPUT TOKENS
    COMMANDs are typically formed the same way that you would normally in the
    shell, only that you will replace your input arguments with placeholder
    tokens like {}, {.}, {/}, {//} and {/.}. If no tokens are provided, it is
    inferred that the final argument in the command will be {}. These tokens
    will perform text manipulation on the inputs to mangle them in the way you
    like. Ideas for more tokens are welcome.

    -   {}: Will supply the input argument untouched.
    -  {.}: Will remove the extension from the input.
    -  {/}: Displays the base name (file name) of the input.
    - {//}: Displays the directory name of the input.
    - {/.}: Displays the base name with the extension removed.
    -  {#}: Displays the current job ID as a number counting from 1.
    - {#^}: Displays the total number of jobs to be processed.
    -  {%}: Displays the thread's ID number.

OPTIONS
    Options may also be supplied to the program to change how the program
    operates:

    -j, --jobs:
        Defines the number of tasks to process in parallel.
        Values may be written as a number (12) or as a percent (150%).
        The default value is the number of CPU cores in the system.

    -u, --ungroup:
        Ungroups the standard output and error to boost the performance of the
        load balancer. This will cause the outputs of each task to be mixed
        together, which may or not matter for your use case.

    -n, --no-shell:
        Allow Parallel to act as the interpreter of the commands for a
        significant performance boost. The downside to this is that you
        can only execute one command at a time.

    -v, --verbose:
        Print information about running processes.

    --num-cpu-cores:
        A convenience command that will print the number of CPU cores in the
        system.

EXAMPLES
    # Command followed by inputs
    parallel -un 'ffmpeg -i {} -c:a libopus -b:a 128k {.}.opus' ::: $(find -type f -name "*.flac")

    # Reading from Stdin
    find -type f -name "*.flac" | parallel -un 'ffmpeg -i {} -c:a libopus -b:a 128k {.}.opus'

    # Inputs are used as commands
    parallel ::: "echo 1" "echo 2" "echo 3" "echo 4"

    # Placeholder values automatically inferred
    parallel -j2 wget ::: URL1 URL2 URL3 URL4

AUTHOR
    Written by Michael Aaron Murphy <mmstickman@gmail.com>
    Inspired by Ole Tange's GNU Parallel.
"#;
