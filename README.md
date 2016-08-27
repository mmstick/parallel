# Parallel: A Command-line CPU Load Balancer Written in Rust
This is an attempt at recreating the functionality of [GNU Parallel](https://www.gnu.org/software/parallel/) in Rust under a MIT license. The end goal will be to support much of the functionality of `GNU Parallel` and then to extend the functionality further for the next generation of command-line utilities written in Rust.

## Benchmark Comparison to GNU Parallel

Here are some benchmarks from an i5-2410M laptop running Ubuntu 16.04.

```sh
parallel 'echo {%}: {}' ::: /usr/bin/* > /dev/null
```

- **GNU Parallel**:
```
real    0m5.911s
user    0m2.752s
sys     0m1.764s
```

- **Rust/MIT Parallel**:
```
real    0m0.559s
user    0m0.084s
sys     0m0.372s
```

## Syntax Examples
The following syntax is supported:

```sh
parallel 'echo {}' ::: *                // {} will be replaced with each input found.
parallel echo ::: *                     // If no placeholders are used, it is automatically assumed.
parallel ::: "echo 1" "echo 2" "echo 3" // If no command is supplied, the input arguments become commands.
```

## Options

In addition to the command syntax, there are also some options that you can use to configure the load balancer:
- **-j**: Defines the number of jobs/threads to run in parallel.
- **--ungroup**: By default, stdout/stderr buffers are grouped in the order that they are received.

Available syntax options for the placeholders values are:
- **{}**: Each occurrence will be replaced with the name of the input.
- **{.}**: Each occurrence will be replaced with the input, with the extension removed.
- **{/}**: Each occurrence will be replaced with the base name of the input.
- **{/.}**: Each occurrence will be replaced with the base name of the input, with the extension removed.
- **{//}**: Each occurrence will be replaced with the directory name of the input.
- **{%}**: Each occurrence will be replaced with the slot number.
- **{#}**: Each occurrence will be replaced with the job number.
- **{#^}**: Each occurrence will be replaced with the total number of jobs.

## Useful Examples

### Transcoding FLAC music to Opus
ffmpeg is a highly useful application for converting music and videos. However, audio transcoding is limited to a
a single core. If you have a large FLAC archive and you wanted to compress it into the efficient Opus codec, it would
take forever with the fastest processor to complete, unless you were to take advantage of all cores in your CPU.

```sh
parallel 'ffmpeg -v 0 -i {} -c:a libopus -b:a 128k {.}.opus' ::: $(find -type f -name '*.flac')
```

### Transcoding Videos to VP9
VP9 has one glaring flaw in regards to encoding: it can only use about three cores at any given point in time. If you
have an eight core processor and a dozen or more episodes of a TV series to transcode, you can use the parallel
program to run three jobs at the same time, provided you also have enough memory for that.

```sh
vp9_params="-c:v libvpx-vp9 -tile-columns 6 -frame-parallel 1 -rc_lookahead 25 -threads 4 -speed 1 -b:v 0 -crf 18"
opus_params="-c:a libopus -b:a 128k"
parallel -j 3 "ffmpeg -v 0 -i {} $vp9_params $opus_params -f webm {.}.webm" ::: $(find -type f -name '*.mkv')
```

## How It Works

There are a lot of commands that will take an input and then consume an entire CPU core as it processes the input.
However, sometimes you have dozens, hundreds, or even thousands of files that you want to process.  The standard
solution would be to construct a for loop and run your jobs serially one at a time.  However, this would take forever
with processes that only make use of a single core.  Another solution is to construct the same for loop but to have
your shell run it in the background.  The problem with that solution is that if there are a lot of inputs to process,
you will end locking up your system and crashing your jobs due to OOM (out of memory) errors.

A complicated setup that I have seen people perform is to create as many separate lists or directories as they have CPU
cores, and then manually spinning up a terminal and copying and pasting the same for loop into each one.  The issue with
this approach is that it takes a lot of time to set this up, and because some tasks finish much sooner than others, you
may end up with several cores sitting and waiting because they've completed all of their assigned inputs while other
cores are busy with many more tasks left to perform.

Instead of processing files using a for loop, you can use a load balancer like `parallel` to distribute jobs evenly
to every core in the system, which will only pass new values when a core has finished it's task.  This has the benefit
that you can process inputs chronologically, and because some inputs may finish sooner than others, you can ensure
that every core has a job to process at any given point in time.  Not to mention, it's about as easy to write as a
for loop:

```sh
# This is a for loop
for file in *; do echo $file; done

# This is a parallel version of that for loop
parallel 'echo {}' ::: *
```

## Installation Instructions

There are a number of methods that you can use to install the application. I provide binary packages for AMD64 systems
that are available for download:

### Ubuntu

```sh
wget https://github.com/mmstick/parallel/releases/download/0.1.0/parallel_0.1.0_amd64.deb
sudo dpkg -i parallel_0.1.0_amd64.deb
```

### Everyone Else

```sh
wget https://github.com/mmstick/parallel/releases/download/0.1.0/parallel.tar.xz
tar xf parallel.tar.xz
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

If you would like to install it system-wide.

```sh
wget https://github.com/mmstick/parallel/archive/master.zip
unzip master.zip
cd parallel-master
cargo build --release
sudo install target/release/parallel /usr/local/bin
```
