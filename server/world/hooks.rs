use types::*;

use super::World;

pub trait Hooks {
    fn on_client_create(&mut self, w: &World, cid: ClientId) {}
    fn on_client_destroy(&mut self, w: &World, cid: ClientId) {}
    fn on_client_change_pawn(&mut self,
                             w: &World,
                             cid: ClientId,
                             old_pawn: Option<EntityId>,
                             new_pan: Option<EntityId>) {}
}

pub struct NoHooks;
impl Hooks for NoHooks {}

pub fn no_hooks() -> &'static mut NoHooks {
    static mut NO_HOOKS: NoHooks = NoHooks;
    unsafe { &mut NO_HOOKS }
}
