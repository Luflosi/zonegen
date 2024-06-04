[SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>]::
[SPDX-License-Identifier: GPL-3.0-only]::

# zonegen
## A drop-in replacement for `nsupdate` but it doesn't rewrite your (hand-written, generated or otherwise externally managed) zone files

`zonegen` generates a partial zone file per zone that is to be included in the main zone file via the `$INCLUDE` directive.
The nameserver needs to be told when the zone file changed and the serial number in the zone file needs to be updated.
But `zonegen` does not have the capability to do so. This will instead be handled by a separate daemon (which I have yet to implement).


## Usage
- Set up [zonewatch](https://github.com/Luflosi/zonewatch)
- Create a directory where you would like to store the generated zone files and the SQLite database
- Call `zonegen` with the `--dir` argument and pass the path to the above directory
- Use something like `update add sub.example.org. 300 IN A 1.2.3.4` for adding or changing a record
- Use something like `update delete sub.example.org. IN A` to remove the above record again
- Use the `send` command to commit the changes you've made so far to the database
- Use the `quit` command or <kbd>ctrl</kbd>+<kbd>d</kbd> or <kbd>ctrl</kbd>+<kbd>c</kbd> to exit
- Use the `help` command to see the valid commands
- If you want to be more declarative and want to keep less state, use the `drop` command to first delete the data in every zone (but the list of zones is not deleted)


## Limitations
This program is meant as a drop-in replacement for the `nsupdate` program but the set of implemented features is fairly small at this time.
This program is currently somewhat specific to my use-case but I still expect it to be useful for other people.
If you have a use-case that is not yet covered or you care about any of the TODO items below, feel free to file an issue or even a PR!

`zonegen` does not parse the existing zone file before overwriting it.
Since reading potentially arbitrary zone files would require more effort, all state is stored in an SQLite database instead and the files are recreated from scratch every time.
If a file would get overwritten with identical contents, it is not rewritten.

The command parser is fairly limited at the moment. If you would like to use commands from `nsupdate` that are not yet implemented, please file an issue!

> [!IMPORTANT]
> This is one of my first Rust projects so the code will not look very idiomatic. If you have any suggestions for improvements, please do not hesitate to create an issue or even a PR! 🖤


## TODO
- Add code to revert database migrations if the application is downgraded (e.g. after a NixOS rollback). The reversible migrations of SQLx seem to not work at all for this use-case
- Set Read-Only file permissions for created zone files
- There must be a way to massively simplify the parser


## License
The license is the GNU GPLv3 (GPL-3.0-only).
