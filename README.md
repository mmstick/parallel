# Parallel: A Command-line CPU Load Balancer Written in Rust
This is an attempt at recreating the functionality of [GNU Parallel](https://www.gnu.org/software/parallel/) in Rust under a MIT license. The end goal will be to support much of the functionality of `GNU Parallel` and then to extend the functionality further for the next generation of command-line utilities written in Rust.

## Benchmark Comparison to GNU Parallel

Here are some benchmarks from an i5-2410M laptop running Gentoo.

```sh
time parallel 'echo {#}: {}' ::: /usr/bin/* > /dev/null
```

### **GNU Parallel**:

#### Default Options

```
real	0m5.728s
user	0m2.960s
sys 	0m1.310s
```

#### Executed with the `--ungroup` Option

```
real	0m4.801s
user	0m2.070s
sys  	0m1.290s
```

### **Rust Parallel**:

#### Default Options

```sh
time target/release/parallel 'echo {#}: {}' ::: /usr/bin/* > /dev/null
```

The default options are the slowest options, with all features enabled.

```
real	0m1.198s
user	0m0.130s
sys  	0m0.550s
```

#### Executed with the `--no-shell` option

```sh
time target/release/parallel --no-shell 'echo {#}: {}' ::: /usr/bin/* > /dev/null
```

A significant amount of overhead is caused by executing commands within the platform's preferred shell. On Unix
systems, that shell is `sh`, whereas on Windows it is `cmd`. Disabling shell executing is a good idea if your
command is simple and doesn't require chaining multiple commands.

```
real    0m0.559s
user    0m0.084s
sys     0m0.372s
```

#### Executed with the `--no-shell` and `--ungroup` Option

```sh
time target/release/parallel --no-shell --ungroup 'echo {#}: {}' ::: /usr/bin/* > /dev/null
```

This will achieve utmost optimization at the cost of not having the standard output and error printed in order.

```
real	0m0.575s
user	0m0.060s
sys	    0m0.4.1s

```

## Syntax Examples
The following syntax is supported:

```sh
parallel 'echo {}' ::: *                          // {} will be replaced with each input found.
parallel echo ::: *                               // If no placeholders are used, it is automatically assumed.
parallel echo :::: list1 list2 list3              // Read newline-delimited arguments stored in files.
parallel echo ::: arg1 ::::+ list :::+ arg2       // Interchangeably read arguments from the command line and files.
parallel echo ::: 1 2 3 ::: A B C ::: D E F       // Permutate the inputs.
parallel ::: "echo 1" "echo 2" "echo 3"           // If no command is supplied, the input arguments become commands.
parallel 'cd {}; echo Directory: {}; echo - {}'   // Commands may be chained in the platform\'s shell.
ls | parallel 'echo {}'                           // If no input arguments are supplied, stdin will be read.
```

## Manual

Parallel parallelizes otherwise non-parallel command-line tasks. When
there are a number of commands that need to be executed, which may be
executed in parallel, the Parallel application will evenly distribute
tasks to all available CPU cores. There are three basic methods for how
commands are supplied:

1. A COMMAND may be defined, followed by an  which denotes
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
>    Deontes that the input arguments that follow are input arguments.
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
- **{/}**: Each occurrence will be replaced with the base name of the input.
- **{/.}**: Each occurrence will be replaced with the base name of the input, with the extension removed.
- **{//}**: Each occurrence will be replaced with the directory name of the input.
- **{%}**: Each occurrence will be replaced with the slot number.
- **{#}**: Each occurrence will be replaced with the job number.
- **{#^}**: Each occurrence will be replaced with the total number of jobs.
- **{N}**: Where N is a number, display the associated job number.
- **{N.}**: will remove the extension from the Nth job.
- **{N/}**: Displays the base name (file name) of the Nth job.
- **{N//}**: Displays the directory name of the Nth job.
- **{N/.}**: Displays the base name of the Nth job with the extension removed.

### OPTIONS

Options may also be supplied to the program to change how the program
operates:

- **-h**, **--help**: Prints the manual for the application (recommended to pipe it to `less`).
- **-j**, **--jobs**: Defines the number of jobs/threads to run in parallel.
- **-u**, **--ungroup**: By default, stdout/stderr buffers are grouped in the order that they are received.
- **-n**, **--no-shell**: Disables executing commands within the platform's shell for a performance boost.
    - Double quotes and backslashes are used to allow spaces in inputs, similar to standard shells.
- **-v**, **--verbose**: Prints information about running processes.
- **--num-cpu-cores**: Prints the number of CPU cores in the system and exits.

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

### Ubuntu

Debian packages are provided on the [releases page](https://github.com/mmstick/parallel/releases).
If a release is not available, it's because I haven't built it yet with cargo deb.  

### Everyone Else

```sh
wget https://github.com/mmstick/parallel/releases/download/0.4.1/parallel_0.4.1_amd64.tar.xz
tar xf parallel_0.4.1.tar.xz
sudo install parallel /usr/local/bin
```

## Compiling From Source

All of the dependencies are vendored locally, so it is possible to build the packages without Internet access.

#### First Method

If you would like to install the latest release directly to `~/.cargo/bin` using the official method.

```sh
cargo install parallel
```

#### Second Method

If you would like to install the latest git release:

```sh
cargo install --git https://github.com/mmstick/parallel parallel
```

#### Third Method

If you would like to install it system-wide.

```sh
wget https://github.com/mmstick/parallel/archive/master.zip
unzip master.zip
cd parallel-master
cargo build --release
sudo install target/release/parallel /usr/local/bin
```
