use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

#[derive(Default, Debug)]
pub struct StringPool {
    inner: RefCell<Inner>,
}

#[derive(Default, Debug)]
struct Inner {
    pool: Vec<String>,
    idx_map: HashMap<String, usize>,
}

impl StringPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&self, s: &str) -> StringId<'_> {
        let mut inner_mut = self.inner.borrow_mut();
        if let Some(idx) = inner_mut.idx_map.get(s).cloned() {
            return StringId { pool: self, idx };
        }

        inner_mut.pool.push(s.to_owned());
        let idx = inner_mut.pool.len() - 1;

        inner_mut.idx_map.insert(s.to_owned(), idx);

        StringId { pool: self, idx }
    }

    fn unchecked_get(&self, idx: usize) -> String {
        self.inner.borrow().pool[idx].clone()
    }
}

pub struct StringId<'p> {
    pool: &'p StringPool,
    idx: usize,
}

impl<'p> Clone for StringId<'p> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool,
            idx: self.idx,
        }
    }
}

impl<'p> Hash for StringId<'p> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl<'p> PartialEq for StringId<'p> {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl<'p> Eq for StringId<'p> {}

impl<'p> Debug for StringId<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringId")
            .field("value", &self.to_string())
            .finish()
    }
}

impl<'p> Display for StringId<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <String as Display>::fmt(&self.pool.unchecked_get(self.idx), f)
    }
}
