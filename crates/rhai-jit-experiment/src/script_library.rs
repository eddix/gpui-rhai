//! Bounded lowering cache for script libraries resolved by Rhai itself.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use rhai::grain::{Compiler, SharedProgram};
use rhai::{Module, Shared};

struct Entry {
    // Pin identity: an address must not be recycled while used as a cache key.
    _library: Shared<Module>,
    program: SharedProgram,
    sequence: u64,
}

#[derive(Default)]
pub(crate) struct ScriptLibraries {
    compilations: u64,
    compile_time: Duration,
    entries: BTreeMap<(usize, String), Entry>,
    sequence: u64,
}

impl ScriptLibraries {
    pub(crate) const fn compilation_stats(&self) -> (u64, Duration) {
        (self.compilations, self.compile_time)
    }
    pub(crate) fn get(
        &mut self,
        library: &Shared<Module>,
        source: Option<&str>,
        maximum: usize,
    ) -> SharedProgram {
        self.sequence = self.sequence.saturating_add(1);
        let key = (
            Shared::as_ptr(library) as usize,
            source.unwrap_or_default().to_owned(),
        );
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.sequence = self.sequence;
            return entry.program.clone();
        }
        let started = Instant::now();
        let program = Shared::new(Compiler::new().compile_library(library, source));
        self.compilations = self.compilations.saturating_add(1);
        self.compile_time = self.compile_time.saturating_add(started.elapsed());
        while self.entries.len() >= maximum.max(1) {
            let victim = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.sequence)
                .map(|(key, _)| key.clone());
            if let Some(victim) = victim {
                self.entries.remove(&victim);
            } else {
                break;
            }
        }
        self.entries.insert(
            key,
            Entry {
                _library: library.clone(),
                program: program.clone(),
                sequence: self.sequence,
            },
        );
        program
    }
}
