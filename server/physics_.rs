use data::Data;


pub struct Physics<'d> {
    data: &'d Data,
}

impl<'d> Physics<'d> {
    pub fn new(data: &'d Data) -> Physics<'d> {
        Physics {
            data: data,
        }
    }
}
