use std::collections::{hash_map, HashMap};
use std::path::{Component as PathComponent, Path};
use std::result::Result as StdResult;

use anyhow::Result;

use crate::utils::string_pool::*;

#[derive(Debug)]
pub struct FileSystemIndex<'p> {
    entries: HashMap<u64, Entry<'p>>,
    root_entry: Entry<'p>,
    next_id: u64,
    file_count: usize,

    string_pool: &'p StringPool,
}

impl<'p> FileSystemIndex<'p> {
    pub fn new(string_pool: &'p StringPool) -> Self {
        Self {
            entries: Default::default(),
            root_entry: Entry {
                name: string_pool.intern("/"),
                entry_type: EntryType::new_dir(),
            },
            next_id: 1,
            file_count: 0,
            string_pool,
        }
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }

    pub fn walk_files<F, E>(&self, f: F) -> StdResult<(), E>
    where
        F: FnMut(&str, &str) -> StdResult<(), E>,
    {
        fn recursively_walk<'p, F, E>(
            entries: &HashMap<u64, Entry<'p>>,
            current_entry: &Entry<'p>,
            current_path: &str,
            f: &mut F,
        ) -> StdResult<(), E>
        where
            F: FnMut(&str, &str) -> StdResult<(), E>,
        {
            match &current_entry.entry_type {
                EntryType::File { file_id } => f(current_path, file_id),
                EntryType::Dir { children } => {
                    for child_id in children.values() {
                        let child_entry = entries
                            .get(child_id)
                            .expect("internal state is inconsistent");
                        let child_path = if current_path.is_empty() {
                            child_entry.name.to_string()
                        } else {
                            format!("{current_path}/{}", child_entry.name)
                        };

                        recursively_walk(entries, child_entry, &child_path, f)?;
                    }

                    Ok(())
                }
            }
        }

        let mut f = f;
        recursively_walk(&self.entries, &self.root_entry, "", &mut f)
    }

    pub fn add_file<P>(&mut self, path: P, file_id: String) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut current_entry = &mut self.root_entry;
        if let Some(parent) = path.as_ref().parent() {
            // Get the parent path and create all intermediate paths if needed.
            for component in parent.components() {
                let PathComponent::Normal(component) = component else {
                    return Err(anyhow!(
                        "invalid path, unexpected path component: `{component:?}`"
                    ));
                };
                let Some(component_str) = component.to_str().map(|s| self.string_pool.intern(s))
                else {
                    return Err(anyhow!("unsupported path component, not UTF-8 compatible"));
                };

                let EntryType::Dir { children } = &mut current_entry.entry_type else {
                    return Err(anyhow!(
                        "intermediate parent path`{}` is not a directory",
                        &current_entry.name
                    ));
                };
                let (entry_id, existed) = match children.entry(component_str.clone()) {
                    hash_map::Entry::Occupied(entry_id) => (*entry_id.get(), true),
                    hash_map::Entry::Vacant(vacant_entry_id) => {
                        let entry_id = self.next_id;
                        self.next_id += 1;
                        vacant_entry_id.insert(entry_id);
                        (entry_id, false)
                    }
                };

                if !existed {
                    let entry = Entry {
                        name: component_str,
                        entry_type: EntryType::new_dir(),
                    };
                    self.entries.insert(entry_id, entry);
                }
                current_entry = self
                    .entries
                    .get_mut(&entry_id)
                    .expect("internal state is inconsistent")
            }
        }

        let Some(file_name_str) = path
            .as_ref()
            .file_name()
            .and_then(|p| p.to_str())
            .map(|s| self.string_pool.intern(s))
        else {
            return Err(anyhow!("unsupported file name, not UTF-8 compatible"));
        };

        let EntryType::Dir { children } = &mut current_entry.entry_type else {
            return Err(anyhow!(
                "parent path `{}` is not a directory",
                &current_entry.name
            ));
        };

        let entry_id = self.next_id;
        self.next_id += 1;

        children.insert(file_name_str.clone(), entry_id);

        let entry = Entry {
            name: file_name_str,
            entry_type: EntryType::new_file(file_id),
        };
        self.entries.insert(entry_id, entry);

        self.file_count += 1;

        Ok(())
    }
}

#[derive(Debug)]
struct Entry<'p> {
    name: StringId<'p>,
    entry_type: EntryType<'p>,
}

#[derive(Debug)]
enum EntryType<'p> {
    File {
        file_id: String,
    },
    Dir {
        children: HashMap<StringId<'p>, u64>,
    },
}

impl<'p> EntryType<'p> {
    fn new_file(file_id: String) -> Self {
        EntryType::File { file_id }
    }

    fn new_dir() -> Self {
        EntryType::Dir {
            children: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::collections::HashMap;

    use super::FileSystemIndex;
    use crate::utils::string_pool::StringPool;

    #[test]
    fn it_works() {
        let string_pool = StringPool::new();
        let mut index = FileSystemIndex::new(&string_pool);

        let mut added_files: HashMap<String, String> = HashMap::new();

        let mut assert_add_file = |path: &str, file_id: &str| {
            let res = index.add_file(path, file_id.to_owned());
            added_files.insert(path.to_owned(), file_id.to_owned());
            assert_matches!(res, Ok(()));
        };

        assert_add_file("Library/Cookies/a", "a");
        assert_add_file("Library/Cookies/b", "b");
        assert_add_file("Library/Preferences/com.example.test.plist", "c");

        let res = index.walk_files(|path, file_id| {
            if let Some(expected_file_id) = added_files.remove(path) {
                assert_eq!(file_id, expected_file_id);
            } else {
                return Err(format!("unexpected file: {path}"));
            }
            Ok(())
        });
        assert_matches!(res, Ok(()));
        assert_eq!(added_files.len(), 0);
    }
}
