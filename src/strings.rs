use std::collections::HashMap;

use gc_arena::{Collect, Collection, Gc, GcWeak, Mutation};
use std::collections::hash_map::Entry::{Occupied, Vacant};

pub struct StringInterner<'gc> {
    mc: &'gc Mutation<'gc>,
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

pub fn simple_hash(s: &str) -> u64 {
    // FNV-1a hash
    let fnv_prime = 1099511628211_u64;
    let mut hash = 14695981039346656037_u64;

    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(fnv_prime);
    }

    hash
}

impl<'gc> StringInterner<'gc> {
    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        Self {
            mc,
            strings: HashMap::new(),
        }
    }

    pub fn intern(&mut self, string: &str) -> Gc<'gc, String> {
        let hash = simple_hash(string);
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
