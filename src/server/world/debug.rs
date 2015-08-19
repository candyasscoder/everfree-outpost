use super::World;
use super::EntityAttachment;

macro_rules! bad {
    ($ok:expr, $msg:expr) => { bad!($ok, $msg,) };
    ($ok:expr, $msg:expr, $($extra:tt)*) => {{
        error!(concat!("broken World invariant: ", $msg), $($extra)*);
        $ok = false;
    }};
}

macro_rules! check {
    ($ok:expr, $cond:expr, $msg:expr) => { check!($ok, $cond, $msg,) };
    ($ok:expr, $cond:expr, $msg:expr, $($extra:tt)*) => {
        if !$cond {
            bad!($ok, $msg, $($extra)*);
        }
    };
}

fn check_invariants(w: &World) -> bool {
    // Don't short circuit the "and" - we want to get warnings from the later checks even if the
    // early ones fail.
    check_client_invariants(w)
}

fn check_client_invariants(w: &World) -> bool {
    let mut ok = true;
    for (cid, c) in w.clients.iter() {

        // - c.pawn refers to an existing entity (or be None)
        // - The entity referenced by c.pawn is attached to c
        if let Some(eid) = c.pawn {
            if let Some(e) = w.entities.get(eid) {
                let attach = e.attachment();
                check!(ok, attach == EntityAttachment::Client(cid),
                       "client {} pawn entity {} is wrongly attached to {:?}",
                       cid.unwrap(), eid.unwrap(), attach);
            } else {
                bad!(ok, "client {} pawn entity {} does not exist",
                     cid.unwrap(), eid.unwrap());
            }
        }

        // - Entities listed in c.child_entities are attached to c
        for &eid in c.child_entities.iter() {
            if let Some(e) = w.entities.get(eid) {
                let attach = e.attachment();
                check!(ok, attach == EntityAttachment::Client(cid),
                       "client {} child entity {} is wrongly attached to {:?}",
                       cid.unwrap(), eid.unwrap(), attach);
            } else {
                bad!(ok, "client {} child entity {} does not exist",
                     cid.unwrap(), eid.unwrap());
            }
        }

        // TODO: check StableIdMap invariants
    }
    ok
}
