use crate::db::BackupManifest;
use crate::utils::format_bytes;
use anyhow::{Context, Result, anyhow};
use plist::{Dictionary, Value};
use std::{fs, path::Path};

pub fn print_backup_info(
    backup_dir: &Path,
    manifest_path: &Path,
    manifest: &BackupManifest,
) -> Result<()> {
    let info_plist = load_plist(backup_dir.join("Info.plist"))?;
    let status_plist = load_plist(backup_dir.join("Status.plist"))?;

    let mut rows = Vec::new();
    rows.push(("Backup path", backup_dir.display().to_string()));
    rows.push(("Manifest path", manifest_path.display().to_string()));

    if let Some(status) = status_plist.as_ref() {
        push_row(&mut rows, "Status date", get_date(status, "Date"));
        push_row(
            &mut rows,
            "Status last completed",
            get_date(status, "Last Completed Backup Date"),
        );
        push_row(
            &mut rows,
            "Encrypted",
            get_bool(status, "WasEncrypted").or_else(|| Some("unknown".into())),
        );
    }

    if let Some(info) = info_plist.as_ref() {
        push_row(
            &mut rows,
            "Info last backup",
            get_date(info, "Last Backup Date"),
        );
        push_row(&mut rows, "Device", get_string(info, "Device Name"));
        push_row(&mut rows, "Display name", get_string(info, "Display Name"));
        push_row(&mut rows, "Product name", get_string(info, "Product Name"));
        push_row(
            &mut rows,
            "Product version",
            get_string(info, "Product Version"),
        );
        push_row(&mut rows, "Product type", get_string(info, "Product Type"));
        push_row(
            &mut rows,
            "Target identifier",
            get_string(info, "Target Identifier"),
        );
        push_row(&mut rows, "GUID", get_string(info, "GUID"));
        push_row(&mut rows, "UDID", get_string(info, "Unique Identifier"));
        push_row(
            &mut rows,
            "Serial number",
            get_string(info, "Serial Number"),
        );
        push_row(&mut rows, "IMEI", get_string(info, "IMEI"));
        push_row(&mut rows, "MEID", get_string(info, "MEID"));
        push_row(&mut rows, "Phone number", get_string(info, "Phone Number"));
        push_row(
            &mut rows,
            "iTunes version",
            get_string(info, "iTunes Version"),
        );
    }

    let total_files = manifest
        .file_count()
        .context("failed to count files in manifest")?;
    let total_domains = manifest
        .domain_count()
        .context("failed to count domains in manifest")?;
    let total_size =
        compute_total_size(backup_dir).context("failed to compute total backup size on disk")?;

    rows.push(("Total files", total_files.to_string()));
    rows.push(("Total domains", total_domains.to_string()));
    rows.push(("Total size", format_bytes(total_size)));

    println!("Backup information:");
    for (label, value) in rows {
        println!("{label:24} {value}");
    }

    Ok(())
}

fn load_plist(path: impl AsRef<Path>) -> Result<Option<Dictionary>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }

    let value = Value::from_file(path)
        .with_context(|| format!("failed to read plist: {}", path.display()))?;
    match value {
        Value::Dictionary(dict) => Ok(Some(dict)),
        _ => Err(anyhow!("{} is not a dictionary plist", path.display())),
    }
}

fn get_string(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(|value| value.as_string().map(|s| s.to_owned()))
}

fn get_bool(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(|value| value.as_boolean())
        .map(|value| {
            if value {
                "yes".to_owned()
            } else {
                "no".to_owned()
            }
        })
}

fn get_date(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(|value| value.as_date().map(|dt| dt.to_xml_format()))
}

fn push_row(rows: &mut Vec<(&'static str, String)>, label: &'static str, value: Option<String>) {
    if let Some(value) = value {
        rows.push((label, value));
    }
}

fn compute_total_size(path: &Path) -> Result<u64> {
    fn accumulate(path: &Path, total: &mut u64) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let entry_path = entry.path();
            if metadata.is_dir() {
                accumulate(&entry_path, total)?;
            } else if metadata.is_file() {
                *total += metadata.len();
            }
        }
        Ok(())
    }

    let mut total = 0;
    accumulate(path, &mut total)?;
    Ok(total)
}
