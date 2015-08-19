use std::collections::HashMap;

use types::*;

use timer;


pub struct Extra {
    pub client_view_update_timer: HashMap<ClientId, timer::Cookie>,
    pub entity_physics_update_timer: HashMap<EntityId, timer::Cookie>,
}

impl Extra {
    pub fn new() -> Extra {
        Extra {
            client_view_update_timer: HashMap::new(),
            entity_physics_update_timer: HashMap::new(),
        }
    }
}
