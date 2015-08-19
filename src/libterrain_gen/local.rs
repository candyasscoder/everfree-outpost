pub trait LocalProperty {
    type Summary: Summary;
    type Temporary;

    fn init(&self) -> Self::Temporary;

    fn load(&self,
            tmp: &mut Self::Temporary,
            pid: Stable<PlaneId>,
            cpos: V2,
            summ: &Self::Summary);

    fn generate(&self,
                tmp: &mut Self::Temporary);

    fn save(&self,
            tmp: &Self::Temporary,
            summ: &mut Self::Summary);
}

pub fn generate<P: LocalProperty>(prop: P,
                                  cache: Cache<P::Summary>,
                                  pid: Stable<PlaneId>,
                                  cpos: V2) -> P::Temporary
