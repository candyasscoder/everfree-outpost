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
c:set_y(17)
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

outpost_ffi.callbacks.test = function(client)
    local entity = client:pawn()
    local pos = entity:pos()
    local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
    local target_tile = target / V3.new(32, 32, 32)
    print('target_tile', target_tile)

    local s = client:world():find_structure_at_point(target_tile)

    if s ~= nil and s:template() == 'tree' then
        print('hit a tree')
        s:replace('stump')
    end
end
