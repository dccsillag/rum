# Rum

Rum is a simple tool to easily run and manage long processes in the background.
<!-- TODO more intro text? -->

It is particularly useful for running code via SSH without dealing with the bloat, complexity and terminal reemulation of tools such as Screen or Tmux -- if you disconnect, your code will continue running in the background, and nothing is lost.

## Installation

You can install Rum via Cargo, Rust's package manager:

```sh
cargo install --git https://github.com/dccsillag/rum.git
```

Alternatively, precompiled binaries for various platforms can be found in [the releases page](https://github.com/dccsillag/rum/releases/latest).

# Overview

To start a new run, simply prefix your command with 'rum'. For example, to start a run of `sleep 10`:

```sh
$ rum sleep 10
Started run d00ba0ab-bfad-43b2-a0d6-9aee1eda6e1f
```

Note that this does not interpret shell syntax. So, if you want to, say, execute `seq 10 | wc -l`, then you should use:

```sh
$ rum sh -c 'seq 10 | wc -l'
Started run 740fbf4a-dca2-4144-bade-9188bfb71d22
```

To list runs which were started with rum, you can use the `--list` subcommand:

```sh
$ rum --list  # or just `rum -l`
d00ba0ab [done] sleep 10
         Started Fri Feb 18 22:36:44 2022, Finished Fri Feb 18 22:36:54 2022
740fbf4a [done] sh -c 'seq 10 | wc -l'
         Started Fri Feb 18 22:37:34 2022, Finished Fri Feb 18 22:37:34 2022
```

Each run has an ID which identifies it -- it is printed right after it is initialized, and is also shown (leftmost) in the output of `rum --list`.
This ID is used to manage runs.

To see more information about a run, use the `--info` subcommand:

```sh
~ $ rum --info d00ba0ab  # or just `rum -i d00ba0ab`
Command:   sleep 10
Status:    finished
Exit code: 0 (success)
Started:   Fri Feb 18 22:36:44 2022
Finished:  Fri Feb 18 22:36:54 2022
```

Note how the identifier shown when we started the run is much longer than the one we passed to the `--info` subcommand, yet it still worked; that's because Rum allows for a very handy shorthand: if there is only one ID which starts with the characters you passed it, it will use that ID. So we could actually have run the command above as follows:

```sh
~ $ rum --info d0  # or just `rum -i d0`
Command:   sleep 10
Status:    finished
Exit code: 0 (success)
Started:   Fri Feb 18 22:36:44 2022
Finished:  Fri Feb 18 22:36:54 2022
```

Using the first two characters of an ID is practical and almost always uniquely identifies an ID (when it doesn't, an extra character will do the trick).

Let's start a new run, which will take a very long time:

```
$ rum python -c 'import time; time.sleep(1000)'
Started run 5d7473cd-5ad5-45b8-a0f6-c01c4b3687c2
```

If we do `rum --list` now, we see that it is running:

```
~ $ rum -l
5d7473cd [running] python -c 'import time; time.sleep(1000)'
         Started Fri Feb 18 22:47:10 2022
d00ba0ab [done] sleep 10
         Started Fri Feb 18 22:36:44 2022, Finished Fri Feb 18 22:36:54 2022
740fbf4a [done] sh -c 'seq 10 | wc -l'
         Started Fri Feb 18 22:37:34 2022, Finished Fri Feb 18 22:37:34 2022
```

We might want to interrupt this run.
Rum provides three ways to interrupt a run: `--interrupt`, `--terminate` and `--kill`. `--interrupt` (or `-c`) is the equivalent of hitting Ctrl+C (i.e., a SIGINT signal); `--terminate` (or `-t`) is the equivalent of killing the process' group (akin `kill <PID>`, i.e., SIGTERM); `--kill` (or `-K`) is the equivalent of killing the process' group with signal 9 (akin `kill -9 <PID>`, i.e., SIGKILL).
Rule of thumb: prefer `-c`. If it doesn't work, fallback to `-t`. Use `-K` only if you must, as it absolutely doesn't allow the process to clean itself up.

So let's use `--interrupt` (`-c`) on the Python run:

```
$ rum --interrupt 5d  # or `rum -c 5d`
```

Now, if we look at the output of `rum --list`:

```
d00ba0ab [done] sleep 10
         Started Fri Feb 18 22:36:44 2022, Finished Fri Feb 18 22:36:54 2022
740fbf4a [done] sh -c 'seq 10 | wc -l'
         Started Fri Feb 18 22:37:34 2022, Finished Fri Feb 18 22:37:34 2022
5d7473cd [killed] python -c 'import time; time.sleep(1000)'
         Started Fri Feb 18 22:47:10 2022, Finished Fri Feb 18 22:56:34 2022
```

Note how Rum now displays the Python process, run `5d7473cd`, as `[killed]` (instead of `[running]` or `[done]`). This indicates that the run was killed by a signal, which is indeed what we just did.

If we were to have a run which exited with non-zero exit code (i.e., it errored out):

```
$ rum python -c 'print(not_in_scope)'
Started run 605abbc1-dc36-4a90-bebf-94bc756100e0
$ rum --list  # or `rum -l`
d00ba0ab [done] sleep 10
         Started Fri Feb 18 22:36:44 2022, Finished Fri Feb 18 22:36:54 2022
740fbf4a [done] sh -c 'seq 10 | wc -l'
         Started Fri Feb 18 22:37:34 2022, Finished Fri Feb 18 22:37:34 2022
5d7473cd [killed] python -c 'import time; time.sleep(1000)'
         Started Fri Feb 18 22:47:10 2022, Finished Fri Feb 18 22:56:34 2022
605abbc1 [failed:1] python -c 'print(not_in_scope)'
         Started Fri Feb 18 23:00:40 2022, Finished Fri Feb 18 23:00:40 2022
```

See how this new run is shown with `[failed:1]` -- this indicates that it exited with a non-zero exit code of 1.

You can also view the output of a run with the `--view` (`-v`) subcommand:

```
rum --view <RUN_ID>
# or
rum -v <RUN_ID>
```

For example, if we were to run

```
rum -v 605abbc1
```

We'd be shown the following output:

```
Traceback (most recent call last):
  File "<string>", line 1, in <module>
NameError: name 'not_in_scope' is not defined
```

The `--view` subcommand also works for runs which are still running, and automatically follows output.

<!-- TODO opening output in a pager -->

Finally, after some time, the output of `rum --list` will begin to be a bit cluttered with runs which are no longer of importance. To aid this, there is the `--remove` (or `-r`) subcommand:

```
$ rum --remove 74
Command:   sh -c 'seq 10 | wc -l'
Status:    finished
Exit code: 0 (success)
Started:   Fri Feb 18 22:37:34 2022
Finished:  Fri Feb 18 22:37:34 2022
 
✔ Are you sure you want to delete this run? · yes
Deleted.
```

It prints information about the given run and asks for confirmation before removing it.

You can also pass it multiple runs at once:

```
~ $ rum --remove 5d 60
Command:   python -c 'import time; time.sleep(1000)'
Status:    finished
Exit code: none (killed)
Started:   Fri Feb 18 22:47:10 2022
Finished:  Fri Feb 18 22:56:34 2022
 
✔ Are you sure you want to delete this run? · yes
Deleted.
Command:   python -c 'print(not_in_scope)'
Status:    finished
Exit code: 1 (failed)
Started:   Fri Feb 18 23:00:40 2022
Finished:  Fri Feb 18 23:00:40 2022
 
✔ Are you sure you want to delete this run? · yes
Deleted.
```

And, if we were to run `rum --list` now, we are left with only our first `sleep 10` run.

```
~ $ rum --list
d00ba0ab [done] sleep 10
         Started Fri Feb 18 22:36:44 2022, Finished Fri Feb 18 22:36:54 2022
```
