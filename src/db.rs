use std::collections::HashMap;
use std::path::Path;

use anyhow::{Error as AnyhowError, Result};
use fallible_iterator::FallibleIterator;
use rusqlite::Connection as SqliteConnection;

pub struct BackupManifest {
    db_conn: SqliteConnection,
}

impl BackupManifest {
    pub fn open<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        if !path.as_ref().exists() {
            return Err(anyhow!(
                "file not exists: {}",
                path.as_ref().to_string_lossy()
            ));
        }

        let db_conn = SqliteConnection::open(path)?;

        // Verify the table schema.
        let mut stmt = db_conn.prepare("PRAGMA table_info('files')")?;
        let rows = stmt.query([])?;
        let mut cols_to_check = HashMap::from([
            ("fileID".to_owned(), "TEXT"),
            ("domain".to_owned(), "TEXT"),
            ("relativePath".to_owned(), "TEXT"),
            ("flags".to_owned(), "INTEGER"),
            ("file".to_owned(), "BLOB"),
        ]);
        rows.map(|r| {
            let name: String = r.get(1)?;
            let typ: String = r.get(2)?;
            Ok((name, typ))
        })
        .map_err(AnyhowError::from)
        .for_each(|r| {
            let Some(expected_type) = cols_to_check.get(&r.0) else {
                return Ok(());
            };
            if *expected_type != r.1 {
                return Err(anyhow!(
                    "column type is not matched, expected `{}` but got `{}`",
                    expected_type,
                    r.1
                ));
            }
            cols_to_check.remove(&r.0);

            Ok(())
        })?;
        drop(stmt);

        if !cols_to_check.is_empty() {
            return Err(anyhow!("table schema is not compatible"));
        }

        Ok(Self { db_conn })
    }

    pub fn query_domains(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .db_conn
            .prepare("SELECT domain FROM files GROUP BY domain")?;
        let rows = stmt.query([])?;
        Ok(rows.map(|r| r.get(0)).collect()?)
    }

    pub fn query_files(&self, domain: &str) -> Result<Vec<ManifestFile>> {
        let mut stmt = self
            .db_conn
            .prepare("SELECT fileID, relativePath, flags, file FROM files WHERE domain = ?")?;
        let rows = stmt.query([domain])?;
        rows.map(|r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(AnyhowError::from)
            .map(|(file_id, relative_path, flags, file)| {
                let file_buf: Vec<u8> = file;
                // TODO: parse metadata from the plist.
                let _file_plist: plist::Value = plist::from_bytes(&file_buf)?;

                let flags: u64 = flags;
                Ok(ManifestFile {
                    file_id,
                    relative_path,
                    file_type: TryFrom::try_from(flags)
                        .map_err(|_| anyhow!("unknown file type: {flags}"))?,
                    file_buf,
                })
            })
            .collect()
    }

    pub fn delete_domain(&self, domain: &str) -> Result<()> {
        let mut stmt = self.db_conn.prepare("DELETE FROM files WHERE domain = ?")?;
        stmt.execute([domain])?;
        Ok(())
    }

    pub fn insert_file(&self, domain: &str, file: &ManifestFile) -> Result<()> {
        let mut stmt = self.db_conn.prepare(
            "INSERT INTO files (fileID, domain, relativePath, flags, file) VALUES (?, ?, ?, ?, ?)",
        )?;
        stmt.execute((
            &file.file_id,
            domain,
            &file.relative_path,
            u64::from(file.file_type),
            &file.file_buf,
        ))?;
        Ok(())
    }
}

#[readonly::make]
#[derive(Debug)]
pub struct ManifestFile {
    pub file_id: String,
    pub relative_path: String,
    pub file_type: ManifestFileType,
    pub file_buf: Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ManifestFileType {
    File,
    Directory,
    SymbolicLink,
}

impl TryFrom<u64> for ManifestFileType {
    type Error = &'static str;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::File,
            2 => Self::Directory,
            4 => Self::SymbolicLink,
            _ => return Err("unknown type"),
        })
    }
}

impl From<ManifestFileType> for u64 {
    fn from(value: ManifestFileType) -> Self {
        match value {
            ManifestFileType::File => 1,
            ManifestFileType::Directory => 2,
            ManifestFileType::SymbolicLink => 4,
        }
    }
}
