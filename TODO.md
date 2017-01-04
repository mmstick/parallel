# Parallel Todo List
The list is actively updated with each successful pull request.

- Implement `disable-temp-files`
    - Re-integrate the original memory-buffered implementation
- Implement `retries`, `resume`, `resume-failed`, and `retry-failed`
    - Will require comparing the processed and unprocessed files
    - Generate a new unprocessed file and remove the originals\
- Implement `workdir` and `tempdir`
    - Allow the ability to change the default location of the temp and work directories
- Fix quoting issues
    - `shell-quote` should infer `dry-run`
    - input tokens should be quoted
- Fix `timeout` for commands that are running within a shell
- Allow the `timeout` parameter to be a percent of the average runtime.
- Eliminate the need to run commands within a shell
- Improve `eta` and implement `progress`
- Fix `-n` issue when using `{1..}` tokens

## May or may not implement
- Kill the youngest job and add it to the back of the queue if available memory is 50% less than `memfree`'s value.
- Implement `compress` to compress outputs
