use std::fs::File;
use std::io;

use types::*;


use terrain_gen::cache::{Cache, Summary};

pub trait LocalProperty {
    type Summary: Summary;
    type Temporary;

    /// Create a new instance of temporary storage for this property.
    fn init(&mut self) -> Self::Temporary;

    /// Load data from an adjacent chunk's summary into temporary storage.
    fn load(&mut self,
            tmp: &mut Self::Temporary,
            dir: V2,
            summ: &Self::Summary);

    /// Generate data for the current chunk and write it to temporary storage.
    fn generate(&mut self,
                          tmp: &mut Self::Temporary);

    /// Copy data from temporary storage into the summary for the current chunk.
    fn save(&mut self,
            tmp: &Self::Temporary,
            summ: &mut Self::Summary);

    /// Generate a chunk summary into the named cache.
    fn generate_into(&mut self,
                     cache: &mut Cache<Self::Summary>,
                     pid: Stable<PlaneId>,
                     cpos: V2) -> Self::Temporary {
        let mut tmp = self.init();

        for &dir in &DIRS {
            unwrap_or!(cache.load(pid, cpos + dir).ok(), continue);
            let summ = cache.get(pid, cpos + dir);
            self.load(&mut tmp, dir, summ);
        }

        self.generate(&mut tmp);

        let summ =
            if let Ok(_) = cache.load(pid, cpos) {
                cache.get_mut(pid, cpos)
            } else {
                cache.create(pid, cpos)
            };
        self.save(&tmp, summ);

        tmp
    }
}

static DIRS: [V2; 8] = [
    V2 { x:  1, y:  0 },
    V2 { x:  1, y:  1 },
    V2 { x:  0, y:  1 },
    V2 { x: -1, y:  1 },
    V2 { x: -1, y:  0 },
    V2 { x: -1, y: -1 },
    V2 { x:  0, y: -1 },
    V2 { x:  1, y: -1 },
];
