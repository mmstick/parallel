extern crate permutate;
mod arguments;
mod buffer;
mod man;
use arguments::InputError;
use buffer::StdoutBuffer;
use buffer::platform::BUFFER_SIZE;
use permutate::Permutator;
use std::io::{self, StdoutLock, Write};
use std::process::exit;

fn main() {
    // First, the program should grab a handle to stdout and stderr and lock them.
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Locking the buffers will improve performance greatly due to not needing
    // to worry about repeatedly locking and unlocking them throughout the program.
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();

    let (input, benchmark, interpret_files, no_delimiters) =
        arguments::parse_options(&mut stdout);

    let mut list_vector = Vec::new();
    match arguments::parse_arguments(&mut list_vector, &input.join(" "), interpret_files) {
        Ok(_) => {
            // Convert the Vec<Vec<String>> into a Vec<Vec<&str>>
            // Convert the Vec<Vec<&str>> into a Vec<&[&str]>
            // And then convert the `Permutator` with the &[&[&str]] as the input.
            let tmp: Vec<Vec<&str>> = list_vector.iter()
                .map(|list| list.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
                .collect();
            let list_array: Vec<&[&str]> = tmp.iter().map(AsRef::as_ref).collect();
            let mut permutator = Permutator::new(&list_array[..]);

            if benchmark {
                let _ = permutator.count();
            } else {
                if no_delimiters {
                    permutate_without_delims(&mut stdout, &mut permutator);
                } else {
                    permutate(&mut stdout, &mut permutator);
                }
            }
        },
        Err(why) => {
            let _ = stderr.write(b"permutate: parse error: ");
            match why {
                InputError::FileError(path, why) => {
                    let _ = stderr.write(path.as_bytes());
                    let _ = stderr.write(b" could not be read: ");
                    let _ = stderr.write(why.as_bytes());
                    let _ = stderr.write(b".\n");
                }
                InputError::NoInputsProvided => {
                    let _ = stderr.write(b"no input was provided after separator.\n");
                },
                InputError::NotEnoughInputs  => {
                    let _ = stderr.write(b"not enough inputs were provided.\n");
                },
            }
            let _ = stderr.write(b"Example Usage: permutate 1 2 3 ::: 4 5 6 ::: 1 2 3\n");
            exit(1);
        }
    }
}

fn permutate(stdout: &mut StdoutLock, permutator: &mut Permutator<str>) {
    let mut buffer = StdoutBuffer::new();
    // This first run through will count the number of bytes that will be
    // required to print each permutation to standard output.
    {
        let current_permutation = permutator.next().unwrap();
        let mut current_permutation = current_permutation.iter();
        buffer.write(current_permutation.next().unwrap().as_bytes());
        buffer.push(b' ');
        buffer.write(current_permutation.next().unwrap().as_bytes());
        for element in current_permutation {
            buffer.push(b' ');
            buffer.write(element.as_bytes())
        }
    }

    buffer.push(b'\n');

    // Using the number of bytes of the first iteration, we can calculate
    // how many iterations that we can safely fit into our buffer.
    let permutations_per_buffer = BUFFER_SIZE / buffer.capacity;

    // Each permutation will check to see if the max number of permutations per
    // buffer has been allocated and prints it to standard output if true.
    let mut counter = 1;
    for permutation in permutator {
        if counter == permutations_per_buffer {
            buffer.write_and_clear(stdout);
            counter = 0;
        }

        // The first element will print a space after the element.
        let mut current_permutation = permutation.iter();
        buffer.write(current_permutation.next().unwrap().as_bytes());
        buffer.push(b' ');
        buffer.write(current_permutation.next().unwrap().as_bytes());
        for element in current_permutation {
            buffer.push(b' ');
            buffer.write(element.as_bytes())
        }
        buffer.push(b'\n');
        counter += 1;
    }

    // Print the remaining buffer to standard output.
    let _ = stdout.write_all(&buffer.data[..]);
}

fn permutate_without_delims(stdout: &mut StdoutLock, permutator: &mut Permutator<str>) {
    // This first run through will count the number of bytes that will be
    // required to print each permutation to standard output.
    let mut buffer = StdoutBuffer::new();
    {
        // There will always be at least two elements in a permutation.
        let permutation     = permutator.next().unwrap();
        let mut permutation = permutation.iter();
        buffer.write(permutation.next().unwrap().as_bytes());
        buffer.write(permutation.next().unwrap().as_bytes());
        for element in permutation { buffer.write(element.as_bytes()); }
    }

    // Append a newline after each permutation to print them on separate lines.
    buffer.push(b'\n');

    // Using the number of bytes of the first iteration, we can calculate
    // how many iterations that we can safely fit into our buffer.
    let permutations_per_buffer = BUFFER_SIZE / buffer.capacity;

    // Each permutation will check to see if the max number of permutations per
    // buffer has been allocated and prints it to standard output if true.
    let mut counter = 1;
    for permutation in permutator {
        let mut permutation = permutation.iter();
        if counter == permutations_per_buffer {
            buffer.write_and_clear(stdout);
            counter = 0;
        }

        // There will always be at least two elements in a permutation.
        buffer.write(permutation.next().unwrap().as_bytes());
        buffer.write(permutation.next().unwrap().as_bytes());
        for element in permutation { buffer.write(element.as_bytes()); }
        buffer.push(b'\n');
        counter += 1;
    }

    // Print the remaining buffer to standard output.
    let _ = stdout.write_all(&buffer.data[..]);
}
