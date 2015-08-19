use rand::Rng;

use libphysics::CHUNK_SIZE;
use libserver_types::*;
use libserver_util as util;

use StdRng;
use dsc::DscGrid;
use pattern::PatternGrid;
use prop::LocalProperty;

use super::provider;
use super::summary::ChunkSummary;


pub struct CliffVaults<'a> {
    rng: StdRng,
    height_grid: &'a DscGrid,
}

impl<'a> CliffVaults<'a> {
    pub fn new(rng: StdRng, height_grid: &'a DscGrid) -> CliffVaults<'a> {
        CliffVaults {
            rng: rng,
            height_grid: height_grid,
        }
    }
}

pub struct Temporary {
    loaded_chunks: [bool; 3 * 3],
    pattern_grid: PatternGrid<u32>,
    entrances: Vec<V3>,
    ramps: Vec<V3>,
}

impl Temporary {
    fn check_placement(&self, pos: V2, size: V2) -> bool {
        // `expand` before the check means we not only reject placement that overlaps other chunks,
        // but also placement that is too close to the edge of the center chunk.  This is a
        // temporary measure to deal with the fact that cave entrances placed right at the chunk
        // edge may not get a chance to influence cave generation (and placing an entrance where
        // there is no cave causes a crash). 
        let area = Region::new(pos - size, pos).expand(scalar(1));
        let chunk_area = area.div_round(CHUNK_SIZE);
        if chunk_area != Region::new(scalar(1), scalar(2)) {
            // TODO: For now, just reject any candidate that extends beyond the center
            // chunk.  Later, fix to allow candidates that extend into not-yet-generated
            // chunks, and add extra constraints as needed (like for trees).
            return false;
        }
        /*
        if !chunk_area.contains(scalar(1)) {
            // Discard candidates that don't at least partially overlap the center chunk.
            return false;
        }

        let chunk_bounds = Region::new(scalar(0), scalar(3));
        for cpos in chunk_area.points() {
            if self.loaded_chunks[chunk_bounds.index(cpos)] {
                // We can't modify already-generated chunks, so discard candidates that
                // would extend into those chunks.
                return false;
            }

            if 
        }
        */

        true
    }
}

impl<'a> LocalProperty for CliffVaults<'a> {
    type Summary = ChunkSummary;
    type Temporary = Temporary;

    fn init(&mut self, _: &ChunkSummary) -> Temporary {
        Temporary {
            loaded_chunks: [false; 3 * 3],
            pattern_grid: PatternGrid::new(scalar(CHUNK_SIZE * 3 + 1), 2, V2::new(4, 4)),
            entrances: Vec::new(),
            ramps: Vec::new(),
        }
    }

    fn load(&mut self, tmp: &mut Temporary, dir: V2, _summ: &ChunkSummary) {
        let bounds = Region::new(scalar(0), scalar(3));
        tmp.loaded_chunks[bounds.index(dir + scalar(1))] = true;
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        for layer in 0 .. CHUNK_SIZE as u8 / 2 {
            let cutoff = provider::cutoff(layer);
            tmp.pattern_grid.init(|pos| {
                let val = self.height_grid.get_value(pos).unwrap();

                let above = val >= cutoff;
                let below = val < cutoff - 2;
                (above as u32) | ((below as u32) << 1)
            });

            let mut candidates = tmp.pattern_grid.find(RAMP_PATTERN, RAMP_MASK);
            util::filter_in_place(&mut candidates, |&pos| {
                tmp.check_placement(pos, V2::new(3, 3))
            });
            let ramp_pos = self.rng.choose(&candidates).map(|&x| x);
            if let Some(pos) = ramp_pos {
                tmp.ramps.push((pos - scalar(CHUNK_SIZE)).extend(layer as i32 * 2));
            }

            let ramp_area = ramp_pos.map(|pos| Region::new(pos - V2::new(3, 3), pos));

            let mut candidates = tmp.pattern_grid.find(ENTRANCE_PATTERN, ENTRANCE_MASK);
            util::filter_in_place(&mut candidates, |&pos| {
                if let Some(ramp_area) = ramp_area {
                    let entrance_area = Region::new(pos - V2::new(3, 1), pos);
                    if entrance_area.overlaps(ramp_area) {
                        return false;
                    }
                }
                tmp.check_placement(pos, V2::new(3, 1))
            });
            let entrance_pos = self.rng.choose(&candidates).map(|&x| x);
            if let Some(pos) = entrance_pos {
                tmp.entrances.push((pos - scalar(CHUNK_SIZE)).extend(layer as i32 * 2));
            }
        }
    }

    fn save(&mut self, tmp: &Temporary, summ: &mut ChunkSummary) {
        summ.cave_entrances = tmp.entrances.clone();
        summ.natural_ramps = tmp.ramps.clone();
    }
}

// Entrance requirements:
//  >= >  >  >=
//  == == == ==
const ENTRANCE_PATTERN: u32 = (0b_00_01_01_00 <<  8) |
                              (0b_00_00_00_00 <<  0);
const ENTRANCE_MASK: u32 =    (0b_10_11_11_10 <<  8) |
                              (0b_11_11_11_11 <<  0);

// Entrance requirements:
//  *  >  >  *
//  >  == == >
//  == == == ==
//  *  == == *
const RAMP_PATTERN: u32 = (0b_00_01_01_00 << 24) |
                          (0b_01_00_00_01 << 16) |
                          (0b_00_00_00_00 <<  8) |
                          (0b_00_00_00_00 <<  0);
const RAMP_MASK: u32 =    (0b_00_11_11_00 << 24) |
                          (0b_11_11_11_11 << 16) |
                          (0b_11_11_11_11 <<  8) |
                          (0b_00_11_11_00 <<  0);
