use libserver_types::*;
use libserver_config::Data;

use GenStructure;
use algo::cellular::CellularGrid;

pub trait Vault {
    fn pos(&self) -> V2;
    fn size(&self) -> V2;

    fn bounds(&self) -> Region<V2> {
        Region::new(self.pos(), self.pos() + self.size())
    }

    fn connection_points(&self) -> &[V2];

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {}

    fn gen_terrain(&self,
                   data: &Data,
                   terrain: &mut [BlockId],
                   bounds: Region<V2>,
                   layer: u8) {}

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {}
}


pub struct FloorMarking {
    pos: V2,
    template_id: TemplateId,
}

impl FloorMarking {
    pub fn new(pos: V2, template_id: TemplateId) -> FloorMarking {
        FloorMarking {
            pos: pos,
            template_id: template_id,
        }
    }
}

impl Vault for FloorMarking {
    fn pos(&self) -> V2 { self.pos }
    fn size(&self) -> V2 { V2::new(1, 1) }

    fn connection_points(&self) -> &[V2] {
        static POINTS: [V2; 1] = [V2 { x: 0, y: 0 }];
        &POINTS
    }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {
        for offset in Region::<V2>::new(scalar(0), scalar(1)).points() {
            if bounds.contains(self.pos + offset) {
                grid.set_fixed(self.pos + offset - bounds.min, false);
            }
        }
    }

    fn gen_structures(&self,
                      _: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        if bounds.contains(self.pos) {
            structures.push(GenStructure::new((self.pos - bounds.min).extend(layer_z),
                                              self.template_id));
        }
    }
}
