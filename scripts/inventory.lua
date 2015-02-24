local outpost_ffi = require('outpost_ffi')

function outpost_ffi.types.Entity.table.inventory(e, name)
    local extra = e:extra()
    local k = 'inventory_' .. name
    if extra[k] == nil then
        local i, err = e:world():create_inventory()
        i:attach_to_entity(e)
        extra[k] = i
    end
    return extra[k]
end

function outpost_ffi.types.Structure.table.inventory(s, name)
    local extra = s:extra()
    local k = 'inventory_' .. name
    if extra[k] == nil then
        local i, err = s:world():create_inventory()
        i:attach_to_structure(s)
        extra[k] = i
    end
    return extra[k]
end
