use std::fs::File;

use types::*;
use libserver_util::bytes::{ReadBytes, WriteBytes};
use util::now;

use engine::glue::*;
use engine::split::EngineRef;
use logic;
use messages::{ClientResponse, SyncKind};
use wire::{WireWriter, WireReader};
use world::Fragment;
use world::object::*;
use world::save::{ObjectReader, ObjectWriter};


pub fn start_up(mut eng: EngineRef) {
    let world_time =
        if let Some(mut file) = eng.storage().open_misc_file() {
            file.read_bytes().unwrap()
        } else {
            0
        };

    let unix_time = now();
    eng.messages_mut().set_world_time(unix_time, world_time);
    eng.timer_mut().set_world_time(unix_time, world_time);
    eng.borrow().unwrap().now = world_time;

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


pub fn pre_restart(eng: EngineRef) {
    let msg = ClientResponse::ChatUpdate("***\tServer restarting...".to_owned());
    eng.messages().broadcast_clients(msg);
    eng.messages().broadcast_clients(ClientResponse::SyncStatus(SyncKind::Reset));

    {
        info!("recording clients to file...");
        let file = eng.storage().create_restart_file();
        let mut ww = WireWriter::new(file);
        for c in eng.world().clients() {
            let wire_id = match eng.messages().client_to_wire(c.id()) {
                Some(x) => x,
                None => {
                    warn!("no wire for client {:?}", c.id());
                    continue;
                },
            };
            ww.write_msg(wire_id, c.name()).unwrap();
        }
    }
}

pub fn post_restart(mut eng: EngineRef, file: File) {
    info!("retrieving clients from file...");

    let mut wr = WireReader::new(file);
    while let Ok(wire_id) = wr.read_header() {
        let name = wr.read::<String>().unwrap();
        warn_on_err!(logic::client::login(eng.borrow(), wire_id, &name));
    }

    let msg = ClientResponse::ChatUpdate("***\tServer restarted".to_owned());
    eng.messages().broadcast_clients(msg);
}
