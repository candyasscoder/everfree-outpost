use types::*;

use world::World;
use world::{Client, TerrainChunk, Entity, Structure, Inventory};
use world::hooks::Hooks;
use world::object::ObjectRefMut;
use world::ops::{self, OpResult};

macro_rules! define_Fragment {
    ($(
        object $Obj:ident {
            id $Id:ident;
            map $objs:ident;
            lifecycle ($($create_arg:ident: $create_arg_ty:ty),*)
                $create_obj:ident => $create_obj_op:ident
                    [$create_id_name:ident -> $create_id_expr:expr],
                $destroy_obj:ident => $destroy_obj_op:ident,
                $create_obj_hooks:ident, $destroy_obj_hooks:ident;
            lookups [$lookup_id_name:ident -> $lookup_id_expr:expr]
                $get_obj:ident, $obj:ident,
                $get_obj_mut:ident, $obj_mut:ident,
                $get_obj_mut_hooks:ident, $obj_mut_hooks:ident;
            $(stable_ids
                $transient_obj_id:ident;)*
        }
    )*) => {

pub trait Fragment<'d>: Sized {
    fn world(&self) -> &World<'d>;
    fn world_mut(&mut self) -> &mut World<'d>;

    type H: Hooks;
    fn with_hooks<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self::H) -> R;

    $(

    fn $create_obj<'a>(&'a mut self, $($create_arg: $create_arg_ty,)*)
                   -> OpResult<ObjectRefMut<'a, 'd, $Obj, Self>> {
        #![allow(unused_variables)]  // id_expr may not reference id_name
        let $create_id_name = try!(ops::$create_obj_op(self, $($create_arg,)*));
        Ok(ObjectRefMut::new(self, $create_id_expr))
    }

    fn $destroy_obj(&mut self, id: $Id) -> OpResult<()> {
        ops::$destroy_obj_op(self, id)
    }

    fn $get_obj_mut<'a>(&'a mut self, $lookup_id_name: $Id)
                        -> Option<ObjectRefMut<'a, 'd, $Obj, Self>> {
        match self.world().$objs.get($lookup_id_expr) {
            None => return None,
            Some(_) => {},
        }

        Some(ObjectRefMut::new(self, $lookup_id_name))
    }

    fn $obj_mut<'a>(&'a mut self, id: $Id) -> ObjectRefMut<'a, 'd, $Obj, Self> {
        self.$get_obj_mut(id)
            .expect(concat!("no ", stringify!($Obj), " with given id"))
    }

    )*
}

    }
}

process_objects!(define_Fragment!);
