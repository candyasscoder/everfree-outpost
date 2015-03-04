use data::Data;

pub struct TerrainGen<'d> {
    data: &'d Data,
}

impl<'d> TerrainGen<'d> {
    pub fn new(data: &'d Data) -> TerrainGen<'d> {
        TerrainGen {
            data: data,
        }
    }
}
