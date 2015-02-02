print = function(...)
    s = ''
    for i = 1, select('#', ...) do
        x = select(i, ...)
        s = s .. tostring(x) .. '\t'
    end
    io.stderr:write(s .. '\n')
end

function dump_rec(x, n)
    for k,v in pairs(x) do
        if type(v) == 'table' then
            print(n .. tostring(k) .. ':')
            dump_rec(v, n .. '  ')
        else
            print(n .. tostring(k) .. ': ' .. tostring(v))
        end
    end
end

function dump(x)
    if type(x) == 'table' then
        dump_rec(x, '')
    else
        print(x)
    end
end

dump(outpost_ffi)

V3 = outpost_ffi.types.V3.table

a = V3.new(1, 2, 3)
b = V3.new(4, 5, 6)
c = a + b
print(c:x(), c:y(), c:z())

test = V3.new(-1, 2, -3)
c = test:abs()
print(c:x(), c:y(), c:z())
print(c:extract())


function outpost_ffi.types.V3.metatable.__tostring(v)
    return tostring(v:x()) .. ',' .. tostring(v:y()) .. ',' .. tostring(v:z())
end

function outpost_ffi.types.World.metatable.__tostring(x)
    return 'World'
end

function outpost_ffi.types.Client.metatable.__tostring(x)
    return 'Client:' .. tostring(x:id())
end

function outpost_ffi.types.Entity.metatable.__tostring(x)
    return 'Entity:' .. tostring(x:id())
end

function outpost_ffi.types.Structure.metatable.__tostring(x)
    return 'Structure:' .. tostring(x:id())
end


client_extra = {}
entity_extra = {}
structure_extra = {}

function get_or_create(t, k)
    local result = t[k]
    if result == nil then
        t[k] = {}
        result = t[k]
    end
    return result
end

function outpost_ffi.types.Client.table.extra(self)
    return get_or_create(client_extra, self:id())
end

function outpost_ffi.types.Entity.table.extra(self)
    return get_or_create(entity_extra, self:id())
end

function outpost_ffi.types.Structure.table.extra(self)
    return get_or_create(structure_extra, self:id())
end

--function outpost_ffi.types.Inventory.table.extra(self)
--    return get_or_create(inventory_extra, self:id())
--end


function outpost_ffi.callbacks.client_destroyed(id)
    client_extra[id] = nil
end

function outpost_ffi.callbacks.entity_destroyed(id)
    entity_extra[id] = nil
end

function outpost_ffi.callbacks.structure_destroyed(id)
    structure_extra[id] = nil
end

function outpost_ffi.callbacks.inventory_destroyed(id)
    inventory_extra[id] = nil
end


function outpost_ffi.callbacks.test(client)
    local entity = client:pawn()
    local pos = entity:pos()
    local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
    local target_tile = target:pixel_to_tile()
    print('target_tile', target_tile)

    local s = client:world():find_structure_at_point(target_tile)
    print('found s', s)
    if s ~= nil then
        dump{ s_info = {
            template = s:template(),
            pos = s:pos(),
            size = s:size(),
        }}
    end

    if s ~= nil and s:template() == 'tree' then
        print('hit a tree')
        ok, err = s:replace('stump')
        if not ok then print('failed to replace', err) end

        local extra = client:extra()
        extra.trees_kicked = (extra.trees_kicked or 0) + 1
        print("kicked " .. extra.trees_kicked .. " trees")
    end
end
