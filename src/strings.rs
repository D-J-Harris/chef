use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use gc_arena::{Collect, Collection, Gc, GcWeak, Mutation};
use std::collections::hash_map::Entry::{Occupied, Vacant};

pub struct StringInterner<'gc> {
    mc: &'gc Mutation<'gc>,
    hasher: DefaultHasher,
    strings: HashMap<u64, GcWeak<'gc, String>>,
}

unsafe impl<'gc> Collect for StringInterner<'gc> {
    fn needs_trace() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn trace(&self, cc: &Collection) {
        self.strings.trace(cc);
    }
}

impl<'gc> StringInterner<'gc> {
    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        Self {
            mc,
            hasher: DefaultHasher::new(),
            strings: HashMap::new(),
        }
    }

    pub fn intern(&mut self, string: &str) -> Gc<'gc, String> {
        string.hash(&mut self.hasher);
        let hash = self.hasher.finish();
        match self.strings.entry(hash) {
            Occupied(mut e) => match e.get().upgrade(self.mc) {
                Some(string) => string,
                None => {
                    let pointer: Gc<'gc, String> = Gc::new(self.mc, string.into());
                    e.insert(Gc::downgrade(pointer));
                    pointer
                }
            },
            Vacant(e) => {
                let pointer: Gc<'gc, String> = Gc::new(self.mc, string.into());
                e.insert(Gc::downgrade(pointer));
                pointer
            }
        }
    }
}
