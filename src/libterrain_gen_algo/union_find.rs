use std::cell::Cell;
use std::u16;


struct Entry {
    parent: Cell<u16>,
    rank: Cell<u16>,
}

impl Entry {
    fn new(idx: u16) -> Entry {
        Entry {
            parent: Cell::new(idx),
            rank: Cell::new(0),
        }
    }
}

pub struct UnionFind {
    entries: Box<[Entry]>,
}

impl UnionFind {
    pub fn new(count: usize) -> UnionFind {
        assert!(count <= u16::MAX as usize);
        let entries = (0 .. count as u16).map(|i| Entry::new(i))
                                         .collect::<Vec<_>>()
                                         .into_boxed_slice();
        UnionFind {
            entries: entries,
        }
    }

    pub fn find(&self, idx: u16) -> u16 {
        let parent = self.entries[idx as usize].parent.get();
        if parent == idx {
            // The element is its own representative.
            return idx;
        }
        let rep = self.find(parent);
        if rep != parent {
            // Path compression: set `parent` to point directly to the representative.
            self.entries[idx as usize].parent.set(rep);
        }
        rep
    }

    /// Join the groups containing `idx0` and `idx1`.  Afterward, `find(idx0) == find(idx1)`.
    /// Returns `true` if the groups were initially distinct.
    pub fn union(&mut self, idx0: u16, idx1: u16) -> bool {
        let rep0 = self.find(idx0);
        let rep1 = self.find(idx1);

        if rep0 == rep1 {
            // Same representatives -> they're already in the same group
            return false;
        }

        let rank0 = self.entries[rep0 as usize].rank.get();
        let rank1 = self.entries[rep1 as usize].rank.get();
        // Attach the shallower tree to the deeper one.  This keeps tree depth to a minimum.
        if rank0 < rank1 {
            self.entries[rep0 as usize].parent.set(rep1);
        } else if rank1 < rank0 {
            self.entries[rep1 as usize].parent.set(rep0);
        } else {
            // Same rank.  Choose a parent arbitrarily.
            self.entries[rep1 as usize].parent.set(rep0);
            self.entries[rep0 as usize].rank.set(rank0 + 1);
        }

        true
    }
}
