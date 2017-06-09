# Parallel Todo List
The list is actively updated with each successful pull request.

- Implement `retries`, `resume`, `resume-failed`, and `retry-failed`
- Fix `timeout` for commands that are running within a shell
- Allow the `timeout` parameter to be a percent of the average runtime.
- Eliminate the need to run commands within a shell
- Implement `progress`
- Fix `-n` issue when using `{1..}` tokens
- Compress arguments written to the disk with Brotli
- Re-implement in-memory argument passing versus disk-exclusive argument iteration
- Rewrite the arguments module
- Rewrite the error handling logic
- Utilize the crossbeam crate so that strings don't need to be leaked

## May or may not implement
- Kill the youngest job and add it to the back of the queue if available memory is 50% less than `memfree`'s value.
- Implement `compress` to compress outputs
