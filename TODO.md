# Parallel Todo List
The list is actively updated with each successful pull request.

## Tests
- Create tests for all functions.

## Speed
- Investigate starting execution early if {#^} is not in use.

## Improvements
- Rewrite parser to write data into list files instead of vectors.
- Also support writing to the processed file in ungrouped mode, or remove support for ungrouped.

## Features
- Colorize the error messages
- Investigate supporting processing nodes via SSH and PowerShell
- Monitor memory consumption and defer execution of arguments if exceeded.
- Monitor CPU consumption and defer execution of arguments if exceeded.
- Support setting nicety of processes
- Support setting core affinity of processes (default 15)
- Offer retry option to retry failed inputs
- Create own shell syntax to replace the need for a platform shell.
