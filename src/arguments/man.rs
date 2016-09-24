pub const MAN_PAGE: &'static str = r#"NAME
    parallel - a command-line CPU load balancer, aka a work-stealer, written in Rust

SYNOPSIS
    parallel [OPTIONS...] 'COMMAND' <MODE INPUTS...>...
    parallel [OPTIONS...] <MODE 'COMMANDS'...>...
    COMMAND | parallel [OPTIONS...] <MODE INPUTS...>...

DESCRIPTION
    Parallel parallelizes otherwise non-parallel command-line tasks. When
    there are a number of commands that need to be executed, which may be
    executed in parallel, the Parallel application will evenly distribute
    tasks to all available CPU cores. There are three basic methods for how
    commands are supplied:

    1. A COMMAND may be defined, followed by a MODE which denotes
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

INPUT MODES
    Input modes are used to determine whether the following inputs are files
    that contain inputs or inputs themselves. Files with inputs have each
    input stored on a separate line, and each line is considered an entire
    input. When there are multiple collected lists of inputs, each individual
    input list will be permutated together into a single list.

    :::
        Denotes that the input arguments that follow are input arguments.
        Additionally, those arguments will be collected into a new list.

    :::+
        Denotes that the input arguments that follow are input arguments.
        Additionally, those arguments will be added to the current list.

    ::::
        Denotes that the input arguments that follow are files with inputs.
        Additionally, those arguments will be collected into a new list.

    ::::+
        Denotes that the input arguments that follow are files with inputs.
        Additionally, those arguments will be added to the current list.

INPUT TOKENS
    COMMANDs are typically formed the same way that you would normally in the
    shell, only that you will replace your input arguments with placeholder
    tokens like {}, {.}, {/}, {//} and {/.}. If no tokens are provided, it is
    inferred that the final argument in the command will be {}. These tokens
    will perform text manipulation on the inputs to mangle them in the way you
    like. Ideas for more tokens are welcome.

    -    {}: Will supply the input argument untouched.
    -   {.}: Will remove the extension from the input.
    -   {/}: Displays the base name (file name) of the input.
    -  {//}: Displays the directory name of the input.
    -  {/.}: Displays the base name with the extension removed.
    -   {#}: Displays the current job ID as a number counting from 1.
    -  {#^}: Displays the total number of jobs to be processed.
    -   {%}: Displays the thread's ID number.
    -   {N}: Where N is a number, display the associated job number.
    -  {N.}: will remove the extension from the Nth job.
    -  {N/}: Displays the base name (file name) of the Nth job.
    - {N//}: Displays the directory name of the Nth job.
    - {N/.}: Displays the base name of the Nth job with the extension removed.

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

    -q, --quiet:
        Disables printing the standard output of running processes.

    -v, --verbose:
        Print information about running processes.

    --version:
        Print version information.

    --num-cpu-cores:
        A convenience command that will print the number of CPU cores in the
        system.

EXAMPLES
    # Command followed by inputs
    parallel -vun 'ffmpeg -i "{}" -c:a libopus -b:a 128k "{.}.opus"' ::: $(find -type f -name "*.flac")

    # Reading from Stdin
    find -type f -name "*.flac" | parallel -vun 'ffmpeg -i "{}" -c:a libopus -b:a 128k "{.}.opus"'

    # Inputs are used as commands
    parallel ::: "echo 1" "echo 2" "echo 3" "echo 4"

    # Placeholder values automatically inferred
    parallel -j2 wget ::: URL1 URL2 URL3 URL4

    # Reading inputs from files and command arguments
    parallel 'echo {}' :::: list1 list2 ::: $(seq 1 10) :::: list3 list4

HOW IT WORKS
    The Parallel command consists of three phases: parsing, threading, and execution.

    1. Parsing Phase
        A. Arguments are read into a write-only in-memory disk buffer which
           stores inputs into an unprocessed file when the disk buffer is full.

        B. Flags are parsed from the command-line along with the command
           argument.

        C. The command argument is tokenized into primitives that serve as
           placeholders for the input arguments.

        D. An input iterator is created that buffers arguments from the
           unprocessed file into an in-memory read-only disk buffer.

    2. Threading Phase
        A. An atomic reference-counted mutex of the input iterator is created
           and shared among all of the threads.

        B. Threads are created for each of the cores that will be processing.

        C. Threads read from the input iterator as soon as they finish a task
           and begin to process the next task.

    3. Execution Phase
        A. Each thread will generate commands by replacing command tokens with
           the current input argument being processed.

        B. Commands are then executed in a sub-process, with their standard
           output and error piped to the main process.

        C. Messages from processes are sorted and printed in the order that
           inputs were given, as if each command was executed serially.

        D. When processes complete, they are written to the processed file.

        E. Once all processes have been completed, the program exits.

AUTHOR
    Written by Michael Aaron Murphy <mmstickman@gmail.com> under the MIT license.
    Inspired by Ole Tange's GNU Parallel, written in Perl.
"#;
