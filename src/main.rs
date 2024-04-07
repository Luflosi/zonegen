// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

#[macro_use]
extern crate error_chain;

mod errors {
	error_chain! {}
}

use crate::repl::repl;
use clap::Parser;
use errors::Result;

mod db;
mod parse;
mod repl;
mod zone_files;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path to the directory where the zone files will be generated and the SQLite database will be stored
	#[arg(short, long)]
	dir: std::path::PathBuf,
}

async fn run(args: Args) -> Result<()> {
	let pool = db::init(&args.dir).await?;

	repl(&pool).await?;

	db::save_zones(&pool, args.dir).await?;

	Ok(())
}

#[tokio::main]
async fn main() {
	let args = Args::parse();

	if let Err(e) = run(args).await {
		eprintln!("Error: {e}");

		for e in e.iter().skip(1) {
			eprintln!("Caused by: {e}");
		}

		if let Some(backtrace) = e.backtrace() {
			println!("Backtrace: {backtrace:?}");
		}

		std::process::exit(1);
	};
}
