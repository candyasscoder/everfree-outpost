use rand::Rng;

use libserver_types::*;
use libphysics::CHUNK_SIZE;
use libserver_config::Data;
use libserver_config::data::{BlockData, StructureTemplates};
use libserver_config::Storage;

use {GenChunk, GenStructure};
use StdRng;
use cache::Cache;
use prop::{LocalProperty, GlobalProperty};

use super::summary::ChunkSummary;
use super::summary::PlaneSummary;

use super::ENTRANCE_POS;
use super::graph_vertices::GraphVertices;
use super::graph_edges::GraphEdges;
use super::caves::Caves;


pub struct Provider<'d> {
    data: &'d Data,
    rng: StdRng,
    cache: Cache<'d, ChunkSummary>,
    plane_cache: Cache<'d, PlaneSummary>,
}

impl<'d> Provider<'d> {
    pub fn new(data: &'d Data, storage: &'d Storage, rng: StdRng) -> Provider<'d> {
        Provider {
            data: data,
            rng: rng,
            cache: Cache::new(storage, "chunk"),
            plane_cache: Cache::new(storage, "plane"),
        }
    }

    fn load_plane_summary(&mut self,
                          pid: Stable<PlaneId>) {
        if let Err(_) = self.plane_cache.load(pid, scalar(0)) {
            GraphVertices::new(self.rng.gen())
                .generate_into(&mut self.plane_cache, pid, scalar(0));
            GraphEdges::new(self.rng.gen())
                .generate_into(&mut self.plane_cache, pid, scalar(0));
        }
    }

    fn generate_summary(&mut self,
                        pid: Stable<PlaneId>,
                        cpos: V2) {
        self.load_plane_summary(pid);
        let plane_summ = self.plane_cache.get(pid, scalar(0));

        Caves::new(self.rng.gen(), cpos, plane_summ)
            .generate_into(&mut self.cache, pid, cpos);
    }


    pub fn generate(&mut self,
                    pid: Stable<PlaneId>,
                    cpos: V2) -> GenChunk {
        self.generate_summary(pid, cpos);


        let mut gc = GenChunk::new();

        {
            let mut ctx = Context {
                rng: &mut self.rng,
                gc: &mut gc,
                summ: self.cache.get(pid, cpos),
                plane_summ: self.plane_cache.get(pid, scalar(0)),
                cpos: cpos,
                block_data: &self.data.block_data,
                structure_templates: &self.data.structure_templates,
                layer: 7,
            };
            ctx.gen();
        }

        /*
        let bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE));


        // Cave/hill layers


        // Add junk
        for &pos in &plane_summ.vertices {
            if !bounds.contains(pos - base) || pos == ENTRANCE_POS {
                continue;
            }
            let pos = pos - base;

            let choice = self.rng.gen_range(0, 20);
            match choice {
                0 => gen_library(&mut self.rng, &mut gc, summ, pos),
                _ => {},
            }
        }
        */

        gc
    }
}

macro_rules! block_id {
    ($self_:ident, $($t:tt)*) => ($self_.block_data.get_id(&format!($($t)*)))
}

macro_rules! template_id {
    ($self_:ident, $($t:tt)*) => ($self_.structure_templates.get_id(&format!($($t)*)))
}

struct Context<'a> {
    rng: &'a mut StdRng,
    gc: &'a mut GenChunk,
    summ: &'a ChunkSummary,
    plane_summ: &'a PlaneSummary,
    cpos: V2,
    block_data: &'a BlockData,
    structure_templates: &'a StructureTemplates,
    layer: u8,
}

impl<'a> Context<'a> {
    fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), scalar(CHUNK_SIZE))
    }

    fn base(&self) -> V2 {
        self.cpos * scalar(CHUNK_SIZE)
    }

    fn global_bounds(&self) -> Region<V2> {
        self.bounds() + self.base()
    }

    fn grid_bounds(&self) -> Region<V2> {
        Region::new(scalar(0), scalar(CHUNK_SIZE + 1))
    }

    fn layer_z(&self) -> i32 {
        self.layer as i32 * 2
    }


    fn gen(&mut self) {
        self.gen_terrain();
        self.gen_exit();
        self.gen_rooms();
    }

    fn gen_terrain(&mut self) {
        let floor_type = "dirt";
        let layer_z = self.layer_z();

        for pos in self.bounds().points() {
            let cave_key = self.get_cave_key(pos);

            self.gc.set_block(pos.extend(layer_z + 0),
                              block_id!(self, "cave/{}/z0/{}", cave_key, floor_type));
            self.gc.set_block(pos.extend(layer_z + 1),
                              block_id!(self, "cave/{}/z1", cave_key));
        }
    }

    fn gen_exit(&mut self) {
        let exit_pos = ENTRANCE_POS + V2::new(0, -1);
        if self.global_bounds().contains(exit_pos) {
            let gs = GenStructure::new((exit_pos - self.base()).extend(self.layer_z()),
                                       template_id!(self, "dungeon_exit"));
            self.gc.structures.push(gs);
        }
    }

    fn gen_rooms(&mut self) {
        for &pos in &self.plane_summ.vertices {
            if !self.global_bounds().contains(pos) || pos == ENTRANCE_POS {
                continue;
            }

            let pos = pos - self.base();
            let choice = self.rng.gen_range(0, 20);
            match choice {
                0 => self.gen_library(pos),
                1 => self.gen_structure_room(pos, template_id!(self, "fountain")),
                2 => self.gen_structure_room(pos, template_id!(self, "trophy")),
                _ => {},
            }
        }
    }

    fn gen_library(&mut self, room_center: V2) {
        // Library
        let w = self.rng.gen_range(3, 10);
        let h = self.rng.gen_range(2, 6);
        let room_base = room_center - V2::new(w / 2, h);

        for y in 0 .. h {
            for x in 0 .. w {
                let pos = room_base + V2::new(x, y * 2);
                if !self.check_placement(pos, scalar(1)) {
                    continue;
                }

                if self.rng.gen_range(0, 10) < 3 {
                    continue;
                }

                let shelf_type = self.rng.gen_range(0, 10);
                let book_count =
                    if shelf_type < 1 { 2 }
                    else if shelf_type < 3 { 1 }
                    else { 0 };

                let gs = GenStructure::new(pos.extend(self.layer_z()),
                                           template_id!(self, "bookshelf/{}", book_count));
                self.gc.structures.push(gs);
            }
        }

        for off in Region::new(scalar(0), V2::new(w, h)).points() {
            let off = V2::new(off.x, 2 * off.y);
        }
    }

    fn gen_structure_room(&mut self, pos: V2, template_id: TemplateId) {
        let size = self.structure_templates.template(template_id).size;
        if self.check_placement(pos, size.reduce()) {
            let gs = GenStructure::new(pos.extend(self.layer_z()), template_id);
            self.gc.structures.push(gs);
        }
    }


    fn get_vertex_key(&self, pos: V2) -> u8 {
        if !self.summ.cave_walls().get(self.grid_bounds().index(pos)) {
            // Open space inside the dungeon
            2
        } else {
            // Wall
            0
        }
    }

    fn get_cave_key(&self, pos: V2) -> u8 {
        let mut acc_cave = 0;
        for &(dx, dy) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
            let val = self.get_vertex_key(pos + V2::new(dx, dy));
            acc_cave = acc_cave * 3 + val;
        }
        acc_cave
    }

    fn check_placement(&self, pos: V2, size: V2) -> bool {
        for p in Region::new(pos, pos + size).points_inclusive() {
            if !self.grid_bounds().contains(p) {
                return false;
            }

            if self.summ.cave_walls().get(self.grid_bounds().index(p)) == true {
                return false;
            }
        }
        true
    }
}

/*

fn gen_library<R: Rng>(rng: &mut R,
                       gc: &mut GenChunk,
                       summ: &ChunkSummary,
                       room_center: V2) {
    // Library
    let w = rng.gen_range(3, 10);
    let h = rng.gen_range(2, 6);
    let room_base = room_center - V2::new(w / 2, h);

    for y in 0 .. h {
        for x in 0 .. w {
            let pos = room_base + V2::new(x, y * 2);
            if rng.gen_range(0, 10) < 3 {
                continue;
            }

            let shelf_type = rng.gen_range(0, 10);
            let book_count =
                if shelf_type < 1 { 2 }
                else if shelf_type < 3 { 1 }
                else { 0 };
        }
    }

    for off in Region::new(scalar(0), V2::new(w, h)) {
        let off = V2::new(off.x, 2 * off.y);
    }
}
*/
