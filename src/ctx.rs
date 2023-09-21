use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as AnyhowContext, Result};

use crate::db::{BackupManifest, ManifestFileType};
use crate::fs_index::FileSystemIndex;
use crate::utils::string_pool::StringPool;

pub struct Context<'p, 'd> {
    backup_dir: &'p Path,
    manifest: &'d mut BackupManifest,
    copy_mode: bool,
}

impl<'p, 'd> Context<'p, 'd> {
    pub fn new(backup_dir: &'p Path, manifest: &'d mut BackupManifest, copy_mode: bool) -> Self {
        Self {
            backup_dir,
            manifest,
            copy_mode,
        }
    }

    pub fn list_domains(&self) -> Result<Vec<String>> {
        self.manifest.query_domains()
    }

    pub fn extract_file<F>(&self, domain: &str, dest_dir: &Path, progress_cb: F) -> Result<()>
    where
        F: FnMut(ProgressEvent),
    {
        let mut progress_cb = progress_cb;

        let string_pool = StringPool::new();
        let mut file_system_index = FileSystemIndex::new(&string_pool);

        progress_cb(ProgressEvent::Querying);
        let files = self
            .manifest
            .query_files(domain)
            .context("failed to query files from database")?;

        for (idx, file) in files.iter().enumerate() {
            if file.file_type != ManifestFileType::File {
                continue;
            }
            if file.file_id.len() != 40 {
                // TODO: handle this error, maybe the database is corrupted.
                continue;
            }

            file_system_index
                .add_file(&file.relative_path, file.file_id.clone())
                .with_context(|| format!("failed to index file: {file:?}"))?;

            progress_cb(ProgressEvent::Indexing {
                indexed: idx + 1,
                total: files.len(),
            });
        }

        let total_file_count = file_system_index.file_count();
        let mut extracted_file_count = 0;
        file_system_index.walk_files(|path, file_id| -> Result<()> {
            let dest_file_path = dest_dir.join(path);
            let dir = dest_file_path.parent().expect("path should have a parent");
            if !dir.exists() {
                fs::create_dir_all(dir).with_context(|| {
                    format!("failed to create directory: {}", dir.to_string_lossy())
                })?;
            } else if !dir.is_dir() {
                return Err(anyhow!(
                    "file already exists but not a directory: {}",
                    dir.to_string_lossy()
                ));
            }

            self.write_file(&dest_file_path, file_id).with_context(|| {
                format!(
                    "failed to create file: {}",
                    dest_file_path.to_string_lossy()
                )
            })?;

            extracted_file_count += 1;
            progress_cb(ProgressEvent::Extracting {
                extracted: extracted_file_count,
                total: total_file_count,
            });

            Ok(())
        })?;

        Ok(())
    }
}

impl<'p, 'd> Context<'p, 'd> {
    fn write_file(&self, file_path: &Path, file_id: &str) -> Result<()> {
        let original_file_path = self.original_file_path(file_id);

        if self.copy_mode {
            fs::copy(original_file_path, file_path)?;
        } else {
            #[cfg(unix)]
            std::os::unix::fs::symlink(original_file_path, file_path)?;
            #[cfg(windows)]
            panic!("symbolic link mode is not supported on Windows");
        }
        Ok(())
    }

    fn original_file_path(&self, file_id: &str) -> PathBuf {
        let bucket = &file_id[0..2];
        self.backup_dir.join(bucket).join(file_id)
    }
}

#[derive(Debug)]
pub enum ProgressEvent {
    Querying,
    Indexing { indexed: usize, total: usize },
    Extracting { extracted: usize, total: usize },
}
