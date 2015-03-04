use storage::Storage;


pub struct Chunks<'d> {
    storage: &'d Storage,
}

impl<'d> Chunks<'d> {
    pub fn new(storage: &'d Storage) -> Chunks<'d> {
        Chunks {
            storage: storage,
        }
    }
}
