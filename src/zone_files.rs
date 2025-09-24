// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

use atomic_write_file::{AtomicWriteFile, unix::OpenOptionsExt as AtomicOpenOptionsExt};
use color_eyre::eyre::{Result, WrapErr};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt as UnixOpenOptionsExt, path::Path};

pub fn save(zone_name: &str, zone_data: &str, dir: &Path) -> Result<()> {
	let zone_file_name = format!("{zone_name}.zone");
	let zone_file_path = Path::new(&dir).join(zone_file_name);
	let maybe_previous_zone_data = fs::read_to_string(&zone_file_path);
	match maybe_previous_zone_data {
		Ok(previous_zone_data) => {
			if previous_zone_data == zone_data {
				println!("File {} did not change, ignoring", zone_file_path.display());
				return Ok(()); // Nothing to be done
			}
		}
		Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
			// Continue with saving the file
		}
		Err(e) => {
			Err(e).wrap_err_with(|| {
				format!(
					"Cannot read existing zone file {}",
					zone_file_path.display()
				)
			})?;
		}
	}
	println!("File {} changed, saving file...", zone_file_path.display());
	let mut file = AtomicWriteFile::options()
		.preserve_mode(false)
		.preserve_owner(false)
		.mode(0o444) // Only allow reading, not writing
		.open(&zone_file_path)
		.wrap_err_with(|| {
			format!(
				"Cannot open the new zone file {} using AtomicWriteFile",
				zone_file_path.display()
			)
		})?;
	file.write_all(zone_data.as_bytes())
		.wrap_err_with(|| format!("Cannot write to new zone file {}", zone_file_path.display()))?;

	file.commit().wrap_err_with(|| {
		format!(
			"Cannot commit new zone file to the filesystem {}",
			zone_file_path.display()
		)
	})?;
	Ok(())
}
