# MIT/Rust Parallel: A Command-line CPU Load Balancer Written in Rust
[![Crates.io](https://img.shields.io/crates/v/parallel.svg)](https://crates.io/crates/parallel)
[![Tokei SLoC Count](https://tokei.rs/b1/github/mmstick/parallel)](https://github.com/mmstick/parallel)
[![AUR](https://img.shields.io/aur/version/parallel-rust.svg)](https://aur.archlinux.org/packages/parallel-rust/)
[![OpenHub Statistics](https://www.openhub.net/p/rust-parallel/widgets/project_thin_badge?format=gif&ref=Thin+badge)](https://www.openhub.net/p/rust-parallel/)

This is an attempt at recreating the functionality of [GNU Parallel](https://www.gnu.org/software/parallel/), a work-stealer for the command-line, in Rust under a MIT license. The end goal will be to support much of the functionality of `GNU Parallel` and then to extend the functionality further for the next generation of command-line utilities written in Rust. While functionality is important, with the application being developed in Rust, the goal is to also be as fast and efficient as possible.

## Note

See the [to-do list](https://github.com/mmstick/parallel/blob/master/TODO.md) for features and improvements that have yet to be done. If you want to contribute, pull requests are welcome. If you have an idea for improvement which isn't listed in the to-do list, feel free to [email me](mailto:mmstickman@gmail.com) and I will consider implementing that idea.

## Benchmark Comparison to GNU Parallel

### GNU Parallel

#### Printing 1 to 10,000 in parallel

```
~/D/parallel (master) $ seq 1 10000 | time -v /usr/bin/parallel echo > /dev/null
    User time (seconds): 194.73
    System time (seconds): 66.49
    Percent of CPU this job got: 230%
    Elapsed (wall clock) time (h:mm:ss or m:ss): 1:53.08
    Maximum resident set size (kbytes): 16140
```

#### Cat the contents of every binary in /usr/bin

```
~/D/parallel (master) $ time -v /usr/bin/parallel cat ::: /usr/bin/* > /dev/null
    User time (seconds): 71.71
    System time (seconds): 27.67
    Percent of CPU this job got: 222%
    Elapsed (wall clock) time (h:mm:ss or m:ss): 0:44.62
    Maximum resident set size (kbytes): 17576
```

#### Logging echo ::: $(seq 1 1000)

```
~/D/parallel (master) $ time -v /usr/bin/parallel --joblog log echo ::: $(seq 1 1000) > /dev/null
    User time (seconds): 21.27
    System time (seconds): 7.44
    Percent of CPU this job got: 238%
    Elapsed (wall clock) time (h:mm:ss or m:ss): 0:12.05
    Maximum resident set size (kbytes): 16624
```

### Rust Parallel (Built with MUSL target)

It's highly recommend to compile Parallel with MUSL instead of glibc, as this reduces memory consumption in half and doubles performance.

#### Printing 1 to 10,000 in parallel

```
~/D/parallel (master) $ seq 1 10000 | time -v target/release/x86_64-unknown-linux-musl/parallel echo > /dev/null
    User time (seconds): 0.40
	System time (seconds): 2.53
	Percent of CPU this job got: 97%
	Elapsed (wall clock) time (h:mm:ss or m:ss): 0:03.01
    Maximum resident set size (kbytes): 1768
```

#### Cat the contents of every binary in /usr/bin

```
~/D/parallel (master) $ time -v target/release/x86_64-unknown-linux-musl/release/parallel cat ::: /usr/bin/* > /dev/null
    User time (seconds): 1.07
	System time (seconds): 4.40
	Percent of CPU this job got: 191%
	Elapsed (wall clock) time (h:mm:ss or m:ss): 0:02.86
    Maximum resident set size (kbytes): 1844
```

#### Logging echo ::: $(seq 1 1000)

```
~/D/parallel (master) $ time -v target/x86_64-unknown-linux-musl/release/parallel --joblog log echo ::: $(seq 1 1000) > /dev/null
    User time (seconds): 0.02
    System time (seconds): 0.28
    Percent of CPU this job got: 85%
    Elapsed (wall clock) time (h:mm:ss or m:ss): 0:00.36
    Maximum resident set size (kbytes): 1768
```

## Syntax Examples
The following syntax is supported:

```sh
parallel 'echo {}' ::: *                          // {} will be replaced with each input found.
parallel echo ::: *                               // If no placeholders are used, it is automatically assumed.
parallel echo :::: list1 list2 list3              // Read newline-delimited arguments stored in files.
parallel echo ::: arg1 ::::+ list :::+ arg2       // Interchangeably read arguments from the command line and files.
parallel echo ::: 1 2 3 ::: A B C ::: D E F       // Permutate the inputs.
parallel echo '{} {1} {2} {3.}' ::: 1 2 file.mkv  // {N} tokens are replaced by the Nth input argument
parallel ::: "echo 1" "echo 2" "echo 3"           // If no command is supplied, the input arguments become commands.
parallel 'cd {}; echo Directory: {}; echo - {}'   // Commands may be chained in the platform\'s shell.
seq 1 10 | parallel 'echo {}'                     // If no input arguments are supplied, stdin will be read.
seq 1 10 | parallel --pipe cat                    // Piping arguments to the standard input of the given command.
#!/usr/bin/parallel --shebang echo                // Ability to use within a shebang line.
```

## Manual

Parallel parallelizes otherwise non-parallel command-line tasks. When
there are a number of commands that need to be executed, which may be
executed in parallel, the Parallel application will evenly distribute
tasks to all available CPU cores. There are three basic methods for how
commands are supplied:

1. A COMMAND may be defined, followed by an  which denotes
   that all following arguments will be used as INPUTS for the command.

2. If no COMMAND is provided, then the INPUTS will be interpreted as
   COMMANDS.

3. If no INPUTS are provided, then standard input will be read for INPUTS.

Parallel groups the standard output and error of each child process so that
outputs are printed in the order that they are given, as if the tasks were
executed serially in a traditional for loop. In addition, commands are
executed in the platform's preferred shell by default, which is `sh -c` on
Unix systems, and `cmd /C` on Windows. This comes at a performance cost, so
it can be disabled with the --no-shell option.

### INPUT MODES

Input modes are used to determine whether the following inputs are files
that contain inputs or inputs themselves. Files with inputs have each
input stored on a separate line, and each line is considered an entire
input.When there are multiple collected lists of inputs, each individual
input list will be permutated together into a single list.

- **:::**
>    Denotes that the input arguments that follow are input arguments.
>    Additionally, those arguments will be collected into a new list.

- **:::+**
>    Denotes that the input arguments that follow are input arguments.
>    Additionally, those arguments will be added to the current list.

- **::::**
>    Denotes that the input arguments that follow are files with inputs.
>    Additionally, those arguments will be collected into a new list.

- **::::+**
>    Denotes that the input arguments that follow are files with inputs.
>    Additionally, those arguments will be added to the current list.

### INPUT TOKENS

COMMANDs are typically formed the same way that you would normally in the
shell, only that you will replace your input arguments with placeholder
tokens like {}, {.}, {/}, {//} and {/.}. If no tokens are provided, it is
inferred that the final argument in the command will be {}. These tokens
will perform text manipulation on the inputs to mangle them in the way you
like. Ideas for more tokens are welcome.

- **{}**: Each occurrence will be replaced with the name of the input.
- **{.}**: Each occurrence will be replaced with the input, with the extension removed.
- **{^abc...}**: Each occurrence will be replaced with a custom suffix removed
- **{/}**: Each occurrence will be replaced with the base name of the input.
- **{/.}**: Each occurrence will be replaced with the base name of the input, with the extension removed.
- **{/^abc...}**: Each occurrence will be replaced with the base name of the input, with a custom suffix removed.
- **{//}**: Each occurrence will be replaced with the directory name of the input.
- **{%}**: Each occurrence will be replaced with the slot number.
- **{#}**: Each occurrence will be replaced with the job number.
- **{##}**: Each occurrence will be replaced with the total number of jobs.
- **{N}**: Where N is a number, display the associated job number.
- **{N.}**: Will remove the extension from the Nth job.
- **{N^abc...}**: Defines a custom suffix to remove from the Nth job, if found.
- **{N/}**: Displays the base name (file name) of the Nth job.
- **{N//}**: Displays the directory name of the Nth job.
- **{N/.}**: Displays the base name of the Nth job with the extension removed.
- **{N/^abc...}**: Displays the basename of the Nth job, with a custom suffix removed.


### OPTIONS

Options may also be supplied to the program to change how the program
operates:

- **--delay**: Delays starting the next job for N amount of seconds, where the seconds can be fractional.
- **--dry-run**: Prints the jobs that will be run to standard output, without running them.
- **--eta**: Prints the estimated time to complete based on average runtime of running processes.
- **-h**, **--help**: Prints the manual for the application (recommended to pipe it to `less`).
- **-j**, **--jobs**: Defines the number of jobs/threads to run in parallel.
- **--joblog**: Logs job statistics to a designated file as they are completed.
- **--joblog-8601**: Writes the start time in the ISO 8601 format: `YYYY-MM-DD hh:mm:ss`
- **--memfree**: Defines the minimum amount of memory available before starting the next job.
- **-n**, **--max-args**: Groups up to a certain number of arguments together in the same command line.
- **--num-cpu-cores**: Prints the number of CPU cores in the system and exits.
- **-p**, **--pipe**: Instead of supplying arguments as arguments to child processes,
        instead supply the arguments directly to the standard input of each child process.
- **-q**, **--quote**: Escapes the command argument supplied so that spaces, quotes, and slashes are retained.
- **-s**, **--silent**, **--quiet**: Disables printing the standard output of running processes.
- **--shebang**: Grants ability to utilize the parallel command as an interpreter via calling it within a shebang line.
- **--shellquote**: Prints commands that will be executed, with the commands quoted.
- **--tmpdir**: Defines the directory to use for temporary files
- **--timeout**: If a command runs for longer than a specified number of seconds, it will be killed with a SIGKILL.
- **-v**, **--verbose**: Prints information about running processes.
- **--version**: Prints the current version of the application and it's dependencies.

## Useful Examples

### Transcoding FLAC music to Opus
ffmpeg is a highly useful application for converting music and videos. However, audio transcoding is limited to a
a single core. If you have a large FLAC archive and you wanted to compress it into the efficient Opus codec, it would
take forever with the fastest processor to complete, unless you were to take advantage of all cores in your CPU.

```sh
parallel 'ffmpeg -v 0 -i "{}" -c:a libopus -b:a 128k "{.}.opus"' ::: $(find -type f -name '*.flac')
```

### Transcoding Videos to VP9
VP9 has one glaring flaw in regards to encoding: it can only use about three cores at any given point in time. If you
have an eight core processor and a dozen or more episodes of a TV series to transcode, you can use the parallel
program to run three jobs at the same time, provided you also have enough memory for that.

```sh
vp9_params="-c:v libvpx-vp9 -tile-columns 6 -frame-parallel 1 -rc_lookahead 25 -threads 4 -speed 1 -b:v 0 -crf 18"
opus_params="-c:a libopus -b:a 128k"
parallel -j 3 'ffmpeg -v 0 -i "{}" $vp9_params $opus_params -f webm "{.}.webm"' ::: $(find -type f -name '*.mkv')
```

## Installation Instructions

There are a number of methods that you can use to install the application. I provide binary packages for AMD64 systems
that are available for download:

### Gentoo

I have a [personal Gentoo layman overlay](https://github.com/mmstick/mmstick-overlay) that provides this application for installation.

### Arch Linux

A PKGBUILD is available for Arch Linux users from the [AUR](https://aur.archlinux.org/packages/parallel-rust/).

### Everyone Else

```sh
rustup target add x86_64-unknown-linux-musl
wget https://github.com/mmstick/parallel/archive/master.zip
unzip master.zip
cd parallel-master
cargo build --release --target x86_64-unknown-linux-musl
sudo install target/x86_64-unknown-linux-musl/release/parallel /usr/local/bin/parallel
```
