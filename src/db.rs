// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

use crate::zone_files;
use color_eyre::eyre::{Result, WrapErr, eyre};
use futures::StreamExt;
use indoc::{formatdoc, indoc};
use sqlx::{
	Pool, Row, Sqlite, Transaction,
	sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::path::{Path, PathBuf};
use tldextract::TldExtractor;

#[derive(sqlx::FromRow)]
struct Zone {
	id: i64,
	name: String,
}

#[derive(sqlx::FromRow)]
struct ResourceRecord {
	subdomain: String,
	ttl: i64,
	class: String,
	type_: String,
	data: String,
}

pub async fn init(dir: &PathBuf) -> Result<Pool<Sqlite>> {
	let db_file_path = Path::new(dir).join("db.sqlite");
	let connection_options = SqliteConnectOptions::new()
		.filename(&db_file_path)
		.create_if_missing(true)
		.journal_mode(SqliteJournalMode::Wal)
		.optimize_on_close(true, None);

	let pool = SqlitePoolOptions::new()
		.connect_with(connection_options)
		.await
		.wrap_err_with(|| format!("Cannot open database file `{}`", db_file_path.display()))?;

	sqlx::migrate!("./migrations")
		.run(&pool)
		.await
		.wrap_err("Cannot run database migrations")?;

	Ok(pool)
}

fn tld_to_zone_and_subdomain(tld_ext: &TldExtractor, input: &str) -> Result<(String, String)> {
	let tld = tld_ext.extract(input).wrap_err_with(|| {
		format!("Cannot extract the TLD information from the provided domain name: {input}")
	})?;
	let domain = tld
		.domain
		.ok_or_else(|| eyre!("Cannot extract the domain from the provided domain name: {input}"))?;
	let suffix = tld
		.suffix
		.ok_or_else(|| eyre!("Cannot extract the suffix from the provided domain name: {input}"))?;
	let zone = format!("{domain}.{suffix}");
	let subdomain = tld.subdomain.map_or_else(|| "@".to_string(), |v| v);

	Ok((zone, subdomain))
}

pub async fn optionally_create_transaction<'a>(
	pool: &'a Pool<Sqlite>,
	optional_tx: Option<Transaction<'a, Sqlite>>,
) -> Result<Option<Transaction<'a, Sqlite>>> {
	match optional_tx {
		None => {
			let tx = pool.begin().await.wrap_err("Cannot begin transaction")?;
			Ok(Some(tx))
		}
		Some(tx) => Ok(Some(tx)),
	}
}

pub async fn optionally_commit_transaction(
	optional_tx: Option<Transaction<'_, Sqlite>>,
) -> Result<Option<Transaction<'_, Sqlite>>> {
	if let Some(tx) = optional_tx {
		tx.commit().await.wrap_err("Cannot commit transaction")?;
	}
	Ok(None)
}

pub async fn optionally_rollback_transaction(
	optional_tx: Option<Transaction<'_, Sqlite>>,
) -> Result<Option<Transaction<'_, Sqlite>>> {
	if let Some(tx) = optional_tx {
		tx.rollback() // The user didn't "send", so discard the changes
			.await
			.wrap_err("Cannot roll back transaction")?;
		eprintln!("WARNING: discarding changes");
	}
	Ok(None)
}

pub async fn add(
	r: crate::parse::Add<'_>,
	tx: &mut Transaction<'_, Sqlite>,
	tld_ext: &TldExtractor,
) -> Result<()> {
	let (zone, subdomain) = tld_to_zone_and_subdomain(tld_ext, r.name)?;

	let zone_row = sqlx::query(indoc! {"
		INSERT OR IGNORE INTO zones (name) VALUES (?1);
		SELECT id FROM zones WHERE name = ?1;
	"})
	.bind(&zone)
	.fetch_one(&mut **tx)
	.await
	.wrap_err("Cannot SELECT row from zones table")?;

	let zoneid: i64 = zone_row
		.try_get("id")
		.wrap_err("Cannot get id from zones table")?;

	let record_row = sqlx::query(indoc! {"
		SELECT id FROM records WHERE zoneid = ?1 AND subdomain = ?2 AND class = ?3 AND type = ?4;
	"})
	.bind(zoneid)
	.bind(&subdomain)
	.bind(r.class)
	.bind(r.type_)
	.fetch_optional(&mut **tx)
	.await
	.wrap_err("Cannot SELECT row from records table")?;

	match record_row {
		Some(row) => {
			let recordid: i64 = row
				.try_get("id")
				.wrap_err("Cannot get id from records table")?;
			sqlx::query(indoc! {"
				UPDATE records SET ttl = ?2, data = ?3
				WHERE id = ?1;
			"})
			.bind(recordid)
			.bind(r.ttl)
			.bind(r.data)
			.execute(&mut **tx)
			.await
			.wrap_err("Cannot UPDATE row in records table")?;
		}
		None => {
			sqlx::query(indoc! {"
				INSERT INTO records (zoneid, subdomain, ttl, class, type, data)
				VALUES (?1, ?2, ?3, ?4, ?5, ?6);
			"})
			.bind(zoneid)
			.bind(&subdomain)
			.bind(r.ttl)
			.bind(r.class)
			.bind(r.type_)
			.bind(r.data)
			.execute(&mut **tx)
			.await
			.wrap_err("Cannot INSERT row into records table")?;
		}
	}

	Ok(())
}

pub async fn drop(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
	// Empty the database but keep the tables themselves.
	// Otherwise the migrations would get messed up.
	// Don't empty the zones table so that we don't suddenly
	// stop generating a zone file and leave the old zone file behind.
	sqlx::query(indoc! {"
		DELETE FROM records;
	"})
	.execute(&mut **tx)
	.await
	.wrap_err("Cannot DELETE from tables")?;

	Ok(())
}

pub async fn delete(
	r: crate::parse::Delete<'_>,
	tx: &mut Transaction<'_, Sqlite>,
	tld_ext: &TldExtractor,
) -> Result<()> {
	let (zone, subdomain) = tld_to_zone_and_subdomain(tld_ext, r.name)?;

	sqlx::query(indoc! {"
		DELETE FROM records
		WHERE subdomain = ?2 AND class = ?3 AND type = ?4
		AND zoneid = (
			SELECT id
			FROM zones
			WHERE name = ?1
		);
	"})
	.bind(zone)
	.bind(subdomain)
	.bind(r.class)
	.bind(r.type_)
	.execute(&mut **tx)
	.await
	.wrap_err("Cannot DELETE from records table")?;

	Ok(())
}

pub async fn save_zones(pool: &Pool<Sqlite>, dir: PathBuf) -> Result<()> {
	let mut zone_rows = sqlx::query_as::<_, Zone>(indoc! {"
		SELECT id, name FROM zones
		ORDER BY name;
	"})
	.fetch(pool);
	while let Some(maybe_zone_row) = zone_rows.next().await {
		let zone = maybe_zone_row.wrap_err("Cannot get row from zones table")?;

		let mut rows = sqlx::query_as::<_, ResourceRecord>(indoc! {"
			SELECT
				subdomain,
				ttl,
				class,
				type AS type_,
				data
			FROM
				records
			WHERE records.zoneid = ?1
			ORDER BY subdomain, class, type, ttl, data;
		"})
		.bind(zone.id)
		.fetch(pool);

		let mut zone_data = formatdoc! {"
			; This file was automatically generated by zonegen.
			; Do not edit or your changes will be overwritten!

			$ORIGIN {}.
		", zone.name};

		while let Some(maybe_row) = rows.next().await {
			let record = maybe_row.wrap_err("Cannot get row from records table")?;
			let zone_data_line = format!(
				"{: <20} {: >6} {: <3} {: <5} {}\n",
				record.subdomain, record.ttl, record.class, record.type_, record.data
			);
			zone_data.push_str(&zone_data_line);
		}
		zone_files::save(&zone.name, &zone_data, &dir)?;
	}

	Ok(())
}
