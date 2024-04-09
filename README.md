[SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>]::
[SPDX-License-Identifier: GPL-3.0-only]::

# zonegen
## A drop-in replacement for `nsupdate` but it doesn't rewrite your (hand-written, generated or otherwise externally managed) zone files
This program is meant as a drop-in replacement for the `nsupdate` program but the set of implemented features is fairly small at this time.
If you have a use-case that is not yet implemented, please file an issue.

`zonegen` generates a partial zone file per zone that is to be included in the main zone file via the `$INCLUDE` directive.
The nameserver needs to be told when the zone file changed and the serial number in the zone file needs to be updated.
But `zonegen` does not have the capability to do so. This will instead be handled by a separate daemon (which I have yet to implement).


## Usage
- Wait for the separate daemon to be written (as mentioned above) as this program is not super useful on its own.
- Create a directory where you would like to store the generated zone files and the SQLite database
- Call `zonegen` with the `--dir` argument and pass the path to the above directory
- Type `help` to see the valid commands

## Limitations
This program is currently somewhat specific to my use-case but I still expect it to be useful for other people.
If you have a use-case that is not yet covered, feel free to file an issue or even a PR!

`zonegen` does not parse the existing zone file before overwriting it.
Since reading potentially arbitrary zone files would require more effort, all state is stored in an SQLite database instead and the files are recreated from scratch every time.
If a file would get overwritten with identical contents, it is not rewritten.

The command parser is fairly limited at the moment. If you would like to use commands from `nsupdate` that are not yet implemented, please file an issue!

It would be nice to verify the queries at compile time. However I found this to have too many papercuts for my taste.
Here are my reasons for not verifying the queries at compile-time with SQLx:
- Can't indent the SQL queries nicely using the indoc! macro
- The database needs to already exist while compiling, requiring extra steps
- SQLx doesn't know that `INTEGER PRIMARY KEY` always implies `NOT NULL` so the inferred datatype is `<Option<i64>` instead of `i64`. This could be worked around by explicitly adding `NOT NULL`
- The following code does not compile because two parameters are expected while I think only one should be required:
    ```rs
    let q = sqlx::query!("
    INSERT OR IGNORE INTO zones (name) VALUES (?1);
    SELECT id FROM zones WHERE name = ?1;
    ", zone).fetch_one(conn).await?;
    ```

> [!IMPORTANT]
> This is one of my first Rust projects so the code will not look very idiomatic. If you have any suggestions for improvements, please do not hesitate to create an issue or even a PR! 🖤


If you would like to see any of the following TODO items implemented, please file an issue so I know that it is important to someone.

TODO:
- Add code to revert database migrations if the application is downgraded (e.g. after a NixOS rollback). The reversible migrations of SQLx seem to not work at all for this use-case
- Set Read-Only file permissions for created zone files
- There must be a way to massively simplify the parser

## License
The license is the GNU GPLv3 (GPL-3.0-only).
