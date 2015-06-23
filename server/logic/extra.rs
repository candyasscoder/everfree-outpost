use std::collections::HashMap;

use types::*;

use timer;


pub struct Extra {
    pub entity_physics_update_timer: HashMap<EntityId, timer::Cookie>,
}

impl Extra {
    pub fn new() -> Extra {
        Extra {
            entity_physics_update_timer: HashMap::new(),
        }
    }
}
