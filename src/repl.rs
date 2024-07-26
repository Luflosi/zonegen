// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db;
use crate::parse::{
	parse, Command,
	Update::{Add, Delete},
};
use color_eyre::eyre::{Result, WrapErr};
use indoc::printdoc;
use nom::error::convert_error;
use rustyline::{error::ReadlineError, DefaultEditor};
use sqlx::{Pool, Sqlite, Transaction};
use tldextract::{TldExtractor, TldOption};

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

pub async fn repl(pool: &Pool<Sqlite>) -> Result<()> {
	let mut optional_tx: Option<Transaction<Sqlite>> = None;
	let tld_ext = TldExtractor::new(TldOption::default());

	let mut rl = DefaultEditor::new().wrap_err("Cannot create default editor")?;
	loop {
		let readline = rl.readline("❯ ");
		match readline {
			Ok(line) => {
				rl.add_history_entry(line.as_str())
					.wrap_err("Cannot add history entry for readline")?;
				match line.as_str() {
					"" => {} // Ignore empty inputs
					non_empty_line => match parse(non_empty_line) {
						Ok(Command::Help) => {
							let version = VERSION.unwrap_or("unknown");
							printdoc! {"
								zonegen v{version}
								send                      (Send the update request)
								quit                      (Quit, any pending update is not sent)
								help                      (Display this message)
								drop                      (Delete the contents of the database)
								[update] add ....         (Add the given record to the zone)
								[update] del[ete] ....    (Remove the given record(s) from the zone)
							"};
						}
						Ok(Command::Send) => {
							optional_tx = db::optionally_commit_transaction(optional_tx).await?;
						}
						Ok(Command::Quit) => {
							break;
						}
						Ok(Command::Drop) => {
							optional_tx =
								db::optionally_create_transaction(pool, optional_tx).await?;
							let tx = optional_tx
								.as_mut()
								.expect("a transaction should exist here");
							db::drop(tx)
								.await
								.wrap_err("Cannot detete the contents of the database")?;
						}
						Ok(Command::Update(Add(r))) => {
							println!("Add request: {r:?}");
							optional_tx =
								db::optionally_create_transaction(pool, optional_tx).await?;
							let tx = optional_tx
								.as_mut()
								.expect("a transaction should exist here");
							db::add(r, tx, &tld_ext)
								.await
								.wrap_err("Cannot add a record")?;
						}
						Ok(Command::Update(Delete(r))) => {
							println!("Delete request: {r:?}");
							optional_tx =
								db::optionally_create_transaction(pool, optional_tx).await?;
							let tx = optional_tx
								.as_mut()
								.expect("a transaction should exist here");
							db::delete(r, tx, &tld_ext)
								.await
								.wrap_err("Cannot delete a record")?;
						}
						Err(e) => {
							match e {
								// TODO: somehow use wrap_err_with() here?
								nom::Err::Failure(f) => {
									eprintln!("Failure: {f:?}");
								}
								nom::Err::Incomplete(f) => {
									eprintln!("Incomplete: {f:?}");
								}
								nom::Err::Error(f) => {
									// Print a hopefully nicer error message
									eprintln!("{}", convert_error(non_empty_line, f));
								}
							};
						}
					},
				};
			}
			Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
				break;
			}
			Err(e) => {
				Err(e).wrap_err("Readline failed")?;
				break;
			}
		}
	}
	db::optionally_rollback_transaction(optional_tx).await?;

	Ok(())
}
