use std::borrow::ToOwned;

use types::*;
use util::bytes::{ReadBytes, WriteBytes};
use util::now;

use engine::glue::*;
use engine::split::EngineRef;
use logic;
use world::Fragment;
use world::object::*;
use world::save::{ObjectReader, ObjectWriter};


pub fn start_up(mut eng: EngineRef) {
    if let Some(mut file) = eng.storage().open_misc_file() {
        let world_time = file.read_bytes().unwrap();
        let unix_time = now();
        eng.messages_mut().set_world_time(unix_time, world_time);
        eng.timer_mut().set_world_time(unix_time, world_time);
    } else {
        let unix_time = now();
        eng.messages_mut().set_world_time(unix_time, 0);
        eng.timer_mut().set_world_time(unix_time, 0);
    }

    if let Some(file) = eng.storage().open_world_file() {
        let mut sr = ObjectReader::new(file);
        sr.load_world(&mut eng.as_save_read_fragment()).unwrap();
    }

    if let Some(file) = eng.storage().open_plane_file(STABLE_PLANE_LIMBO) {
        let mut sr = ObjectReader::new(file);
        sr.load_plane(&mut eng.as_save_read_fragment()).unwrap();
    } else {
        let name = "Limbo".to_owned();
        let stable_pid = eng.as_hidden_world_fragment().create_plane(name).unwrap().stable_id();
        assert!(stable_pid == STABLE_PLANE_LIMBO);
    }

    if let Some(file) = eng.storage().open_plane_file(STABLE_PLANE_FOREST) {
        let mut sr = ObjectReader::new(file);
        sr.load_plane(&mut eng.as_save_read_fragment()).unwrap();
    } else {
        let name = "Everfree Forest".to_owned();
        let stable_pid = eng.as_hidden_world_fragment().create_plane(name).unwrap().stable_id();
        assert!(stable_pid == STABLE_PLANE_FOREST);
    }
}


pub fn shut_down(mut eng: EngineRef) {
    while let Some(cid) = eng.world().clients().next().map(|c| c.id()) {
        warn_on_err!(logic::client::logout(eng.borrow(), cid));
    }

    while let Some((pid, cpos)) = eng.world().terrain_chunks().next()
                              .map(|tc| (tc.plane_id(), tc.chunk_pos())) {
        logic::chunks::unload_chunk(eng.borrow(), pid, cpos);
    }

    while let Some(pid) = eng.world().planes().next().map(|p| p.id()) {
        logic::chunks::unload_plane(eng.borrow(), pid);
    }

    {
        let (h, eng) = eng.borrow().0.split_off();
        let h = SaveWriteHooks(h);
        let file = eng.storage().create_world_file();
        let mut sw = ObjectWriter::new(file, h);
        warn_on_err!(sw.save_world(eng.world()));
    }

    {
        let mut file = eng.storage().create_misc_file();
        warn_on_err!(file.write_bytes(eng.now()));
    }
}
