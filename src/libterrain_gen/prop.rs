use libserver_types::*;

use cache::{Cache, Summary};


// TODO: remove Self/Temporary distinction
pub trait LocalProperty {
    type Summary: Summary;
    type Temporary;
    type Result;

    /// Create a new instance of temporary storage for this property.
    fn init(&mut self,
            summ: &Self::Summary) -> Self::Temporary;

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
            tmp: Self::Temporary,
            summ: &mut Self::Summary) -> Self::Result;

    /// Generate a chunk summary into the named cache.
    fn generate_into(&mut self,
                     cache: &mut Cache<Self::Summary>,
                     pid: Stable<PlaneId>,
                     cpos: V2) -> Self::Result {
        let mut tmp = {
            let summ =
                if let Ok(_) = cache.load(pid, cpos) {
                    cache.get(pid, cpos)
                } else {
                    &*cache.create(pid, cpos)
                };
            self.init(summ)
        };

        for &dir in &DIRS {
            match cache.load(pid, cpos + dir) {
                Ok(_) => {},
                Err(_) => continue,
            }
            let summ = cache.get(pid, cpos + dir);
            self.load(&mut tmp, dir, summ);
        }

        self.generate(&mut tmp);

        // Summary was previously loaded.  We assume it's still around (requires cache size of at
        // least 9).
        self.save(tmp, cache.get_mut(pid, cpos))
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


pub trait GlobalProperty {
    type Summary: Summary;
    type Temporary;
    type Result;

    /// Create a new instance of temporary storage for this property.
    fn init(&mut self,
            summ: &Self::Summary) -> Self::Temporary;

    /// Generate data for the current chunk and write it to temporary storage.
    fn generate(&mut self,
                tmp: &mut Self::Temporary);

    /// Copy data from temporary storage into the summary.
    fn save(&mut self,
            tmp: Self::Temporary,
            summ: &mut Self::Summary) -> Self::Result;

    /// Generate a chunk summary into the named cache.
    fn generate_into(&mut self,
                     cache: &mut Cache<Self::Summary>,
                     pid: Stable<PlaneId>,
                     cpos: V2) -> Self::Result {
        let mut tmp = {
            let summ =
                if let Ok(_) = cache.load(pid, cpos) {
                    cache.get(pid, cpos)
                } else {
                    &*cache.create(pid, cpos)
                };
            self.init(summ)
        };

        self.generate(&mut tmp);

        self.save(tmp, cache.get_mut(pid, cpos))
    }
}
