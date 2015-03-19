use types::*;

use engine::glue::*;
use engine::split::EngineRef;
use logic;
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};


pub fn start_up(mut eng: EngineRef) {
    if let Some(file) = eng.storage().open_world_file() {
        let mut sr = ObjectReader::new(file);
        sr.load_world(&mut eng.as_save_read_fragment()).unwrap()
    }
}


pub fn shut_down(mut eng: EngineRef) {
    while let Some(cid) = eng.world().clients().next().map(|c| c.id()) {
        warn_on_err!(logic::client::logout(eng.borrow(), cid));
    }

    while let Some(cpos) = eng.world().terrain_chunks().next().map(|t| t.id()) {
        logic::chunks::unload_chunk(eng.borrow(), cpos);
    }

    {
        let (h, eng) = eng.borrow().0.split_off();
        let h = SaveWriteHooks(h);
        let file = eng.storage().create_world_file();
        let mut sw = ObjectWriter::new(file, h);
        warn_on_err!(sw.save_world(eng.world()));
    }
}
