use rand::Rng;

use libserver_types::*;
use libphysics::CHUNK_SIZE;
use libserver_config::Data;
use libserver_config::Storage;

use GenChunk;
use StdRng;
use cache::Cache;
use prop::{LocalProperty, GlobalProperty};

use super::summary::ChunkSummary;
use super::summary::PlaneSummary;

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
        let summ = self.cache.get(pid, cpos);
        let bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE));

        let block_data = &self.data.block_data;
        macro_rules! block_id {
            ($($t:tt)*) => (block_data.get_id(&format!($($t)*)))
        };

        let structure_templates = &self.data.structure_templates;
        macro_rules! template_id {
            ($($t:tt)*) => (structure_templates.get_id(&format!($($t)*)))
        };


        // Cave/hill layers

        let floor_type = "dirt";
        let layer_z = 14;

        for pos in bounds.points() {
            let cave_key = get_cave_key(summ, pos);

            gc.set_block(pos.extend(layer_z + 0),
                         block_id!("cave/{}/z0/{}", cave_key, floor_type));
            gc.set_block(pos.extend(layer_z + 1),
                         block_id!("cave/{}/z1", cave_key));
        }


        // Mark vertices
        let plane_summ = self.plane_cache.get(pid, scalar(0));
        let base = cpos * scalar(CHUNK_SIZE);
        for &pos in &plane_summ.vertices {
            if bounds.contains(pos - base) {
                gc.set_block((pos - base).extend(layer_z),
                             block_id!("grass/center/v0"));
            }
        }

        gc
    }
}

fn get_vertex_key(summ: &ChunkSummary, pos: V2) -> u8 {
    let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE + 1));
    if !summ.cave_walls().get(bounds.index(pos)) {
        // Open space inside the dungeon
        2
    } else {
        // Wall
        0
    }
}

fn get_cave_key(summ: &ChunkSummary, pos: V2) -> u8 {
    let mut acc_cave = 0;
    for &(dx, dy) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
        let val = get_vertex_key(summ, pos + V2::new(dx, dy));
        acc_cave = acc_cave * 3 + val;
    }
    acc_cave
}
