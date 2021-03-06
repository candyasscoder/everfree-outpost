local outpost_ffi = require('outpost_ffi')


-- Extra data associated with userdata.

local world_extra = {}
local client_extra = {}
local entity_extra = {}
local inventory_extra = {}
local plane_extra = {}
local structure_extra = {}

-- Callbacks used for save/load and to clear data when objects are destroyed.

function outpost_ffi.callbacks.get_world_extra()
    return world_extra
end

function outpost_ffi.callbacks.get_client_extra(id)
    return client_extra[id]
end

function outpost_ffi.callbacks.get_entity_extra(id)
    return entity_extra[id]
end

function outpost_ffi.callbacks.get_inventory_extra(id)
    return inventory_extra[id]
end

function outpost_ffi.callbacks.get_plane_extra(id)
    return plane_extra[id]
end

function outpost_ffi.callbacks.get_structure_extra(id)
    return structure_extra[id]
end


function outpost_ffi.callbacks.set_world_extra(extra)
    if extra == nil then
        world_extra = {}
    else
        world_extra = extra
    end
end

function outpost_ffi.callbacks.set_client_extra(id, extra)
    client_extra[id] = extra
end

function outpost_ffi.callbacks.set_entity_extra(id, extra)
    entity_extra[id] = extra
end

function outpost_ffi.callbacks.set_inventory_extra(id, extra)
    inventory_extra[id] = extra
end

function outpost_ffi.callbacks.set_plane_extra(id, extra)
    plane_extra[id] = extra
end

function outpost_ffi.callbacks.set_structure_extra(id, extra)
    structure_extra[id] = extra
end


-- Extension methods for accessing the extra data.

local function get_or_create(t, k)
    local result = t[k]
    if result == nil then
        t[k] = {}
        result = t[k]
    end
    return result
end

function outpost_ffi.types.World.table.extra(self)
    return world_extra
end

function outpost_ffi.types.Client.table.extra(self)
    return get_or_create(client_extra, self:id())
end

function outpost_ffi.types.Entity.table.extra(self)
    return get_or_create(entity_extra, self:id())
end

function outpost_ffi.types.Inventory.table.extra(self)
    return get_or_create(inventory_extra, self:id())
end

function outpost_ffi.types.Plane.table.extra(self)
    return get_or_create(plane_extra, self:id())
end

function outpost_ffi.types.Structure.table.extra(self)
    return get_or_create(structure_extra, self:id())
end


-- Callbacks used when structures are loaded/unloaded

local structure_hooks = {}

local function register_structure_hooks(hooks)
    structure_hooks[#structure_hooks + 1] = hooks
end

function outpost_ffi.callbacks.structure_load(id)
    print('load', id)
    for _, hooks in ipairs(structure_hooks) do
        if hooks.load ~= nil then
            hooks.load(get_or_create(structure_extra, id), id)
        end
    end
end

function outpost_ffi.callbacks.structure_unload(id)
    print('unload', id)
    for _, hooks in ipairs(structure_hooks) do
        if hooks.unload ~= nil then
            hooks.unload(get_or_create(structure_extra, id), id)
        end
    end
end


return {
    register_structure_hooks = register_structure_hooks,
}
