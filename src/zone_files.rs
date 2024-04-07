// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

use crate::errors::{Result, ResultExt};
use std::{fs, io::Write, path::Path};
use tempfile_fast::Sponge;

pub fn save(zone_name: &str, zone_data: &str, dir: &Path) -> Result<()> {
	let zone_file_name = format!("{zone_name}.zone");
	let zone_file_path = Path::new(&dir).join(zone_file_name);
	let maybe_previous_zone_data = fs::read_to_string(&zone_file_path);
	match maybe_previous_zone_data {
		Ok(previous_zone_data) => {
			if previous_zone_data == zone_data {
				println!("File {} did not change, ignoring", zone_file_path.display());
				return Ok(()); // Nothing to be done
			};
		}
		Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
			// Continue with saving the file
		}
		Err(e) => {
			Err(e).chain_err(|| {
				format!(
					"Cannot read existing zone file {}",
					zone_file_path.display()
				)
			})?;
		}
	}
	println!("File {} changed, saving file...", zone_file_path.display());
	let mut temp = Sponge::new_for(&zone_file_path).chain_err(|| {
		format!(
			"Cannot create new Sponge for writing to new zone file {}",
			zone_file_path.display()
		)
	})?;
	temp.write_all(zone_data.as_bytes())
		.chain_err(|| format!("Cannot write to new zone file {}", zone_file_path.display()))?;

	temp.commit().chain_err(|| {
		format!(
			"Cannot commit new zone file to the filesystem {}",
			zone_file_path.display()
		)
	})?;
	Ok(())
}
