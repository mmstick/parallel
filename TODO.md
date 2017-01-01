# Parallel Todo List
The list is actively updated with each successful pull request.

- Implement `disable-temp-files`
    - Re-integrate the original memory-buffered implementation
- Implement `retries`, `resume`, `resume-failed`, and `retry-failed`
    - Will require comparing the processed and unprocessed files
    - Generate a new unprocessed file and remove the originals
- Implement `delay` and `timeout`
    - Add a delay timer between commands and a timeout timer to kill applications that run too long
- Implement `compress`
    - Compress the outputs of a file if the file exceeds a certain length
- Implement `eta` and `progress`
    - Estimate how long it will take for the commands to complete and show a progress bar
- Implement `memfree`
    - Only execute a task if memory consumption is below a certain threshold.
- Implement `skip-first-line` and `shebang`
    - Basically, enale the ability to use parallel in a shebang line
- Implement `workdir` and `tempdir`
    - Allow the ability to change the default location of the temp and work directories
- Fix quoting issues
    - `shell-quote` should infer `dry-run`
    - input tokens should be quoted
