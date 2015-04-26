local sampler = IsoDiskSampler.new_constant(12347, 4, 32)

local function make_ds()
    local offsets = ValuesMut.new()
    for _, v in ipairs({8, 4, 2, 1, 0}) do
        offsets:push(v)
        offsets:push(v)
    end
    return DiamondSquare.new(1234, 5678, RandomField.new(1, 2, -16, 16):upcast(), offsets)
end

local ds = make_ds()
local water = BorderField.new((FilterField.new(make_ds():upcast(), -999, -13):upcast()))
local caves = BorderField.new((FilterField.new(make_ds():upcast(), 13, 999):upcast()))

-- Generated 2015-03-20 07:17:33 by util/gen_border_shape_table.py
local TILE_ID_MAP = {
    'outside',
    'center',
    'edge/n',
    'edge/s',
    'edge/e',
    'edge/w',
    'corner/inner/nw',
    'corner/inner/ne',
    'corner/inner/sw',
    'corner/inner/se',
    'corner/outer/nw',
    'corner/outer/ne',
    'corner/outer/sw',
    'corner/outer/se',
    'cross/nw',
    'cross/ne',
}

local function place_cave(c, cpos, x, y, loot)
    print('placing cave', cpos, x, y)
    c:set_block(V3.new(x - 1, y, 0), 'cave_entrance/x0/z0')
    c:set_block(V3.new(x - 1, y, 1), 'cave_entrance/x0/z1')
    c:set_block(V3.new(x,     y, 0), 'cave_entrance/x1/z0')
    c:set_block(V3.new(x,     y, 1), 'cave_entrance/x1/z1')
    c:set_block(V3.new(x + 1, y, 0), 'cave_entrance/x2/z0')
    c:set_block(V3.new(x + 1, y, 1), 'cave_entrance/x2/z1')

    if loot ~= nil then
        if loot == 'dungeon' then
            c:add_structure(V3.new(x, y - 3, 0), 'dungeon_entrance')
        else
            c:add_structure_with_extras(V3.new(x, y - 3, 0), 'chest', { loot = loot })
        end
    end
end

local function choose_loot(r)
    if r:gen(1, 20) == 1 then
        return 'dungeon'
    end

    local item = r:choose_weighted(pairs({
        wood = 5,
        stone = 5,
        crystal = 15
    }))
    local amount
    if item == 'crystal' then
        amount = r:gen(15, 20)
    else
        amount = r:gen(80, 120)
    end

    if r:gen(1, 50) == 1 then
        return 'H ' .. amount .. ' ' .. item
    else
        return amount .. ' ' .. item
    end
end

local function generate_forest(c, cpos, r)
    local grass = {
        ['grass/center/v0'] = 1,
        ['grass/center/v1'] = 1,
        ['grass/center/v2'] = 1,
        ['grass/center/v3'] = 1,
    }

    local min = cpos * V2.new(16, 16)
    local max = min + V2.new(16, 16)

    local water_border = water:get_region(min, max)
    local cave_border = caves:get_region(min, max)

    local old_cb_1 = 0
    local old_cb_2 = 0
    local placed_entrance = false

    for y = 0, 15 do
        for x = 0, 15 do
            local wb = water_border[y * 16 + x + 1]
            local cb = cave_border[y * 16 + x + 1]
            if wb ~= 0 then
                c:set_block(V3.new(x, y, 0), 'water_grass/' .. TILE_ID_MAP[wb + 1])
            else if cb ~= 0 then
                c:set_block(V3.new(x, y, 0), 'cave/' .. TILE_ID_MAP[cb + 1] .. '/z0')
                c:set_block(V3.new(x, y, 1), 'cave/' .. TILE_ID_MAP[cb + 1] .. '/z1')
                c:set_block(V3.new(x, y, 2), 'cave_top/' .. TILE_ID_MAP[cb + 1])

                if not placed_entrance and x >= 2 and y >= 4 and
                        cb == 3 and old_cb_1 == 3 and old_cb_2 == 3 then
                    local space = 0
                    for i = 1, 3 do
                        local cb2 = cave_border[(y - i) * 16 + x + 1]
                        if cb2 == 1 then
                            space = i
                        else
                            break
                        end
                    end
                    if space >= 3 then
                        place_cave(c, cpos, x - 1, y, choose_loot(r))
                    else if space >= 1 then
                        place_cave(c, cpos, x - 1, y, nil)
                    end end
                    placed_entrance = true
                end
                old_cb_2 = old_cb_1
                old_cb_1 = cb
            else
                c:set_block(V3.new(x, y, 0), r:choose_weighted(pairs(grass)))
            end end
        end
    end

    local structures = {
        ['tree'] = 2,
        ['rock'] = 1,
    }
    local p = sampler:get_points(min, max)

    for i = 1, #p do
        local wb = water:get_region(p[i], p[i] + V2.new(4, 2))
        local cb = caves:get_region(p[i], p[i] + V2.new(4, 2))
        local ok = true
        for j = 1, #wb do
            if wb[j] ~= 0 or cb[j] ~= 0 then
                ok = false
                break
            end
        end
        if ok then
            c:add_structure((p[i] - min):extend(0), r:choose_weighted(pairs(structures)))
        end
    end

    if cpos:x() == 0 and cpos:y() == 0 then
        c:add_structure(V3.new(0, 0, 0), 'anvil')
    end
end


local function place_cave_inside(c, x, y, variant)
    c:set_block(V3.new(x, y, 0), 'cave_inside/' .. variant .. '/z0')
    c:set_block(V3.new(x, y, 1), 'cave_inside/' .. variant .. '/z1')
end

local function fill_grid(grid, x0, y0, x1, y1)
    for y = y0, y1 do
        for x = x0, x1 do
            grid[y + 2][x + 2] = 1
        end
    end
end


local BORDER_LOOKUP = {
    k1000 = 'edge/s',
    k0100 = 'edge/n',
    k0010 = 'edge/e',
    k0001 = 'edge/w',
    k1010 = 'corner/inner/nw',
    k1001 = 'corner/inner/ne',
    k0110 = 'corner/inner/sw',
    k0101 = 'corner/inner/se',
}

local function variant_from_grid(g, x, y)
    if g[y][x] == 1 then
        return 'center'
    end

    local n = g[y - 1][x]
    local s = g[y + 1][x]
    local w = g[y][x - 1]
    local e = g[y][x + 1]

    local key = 'k' .. n .. s .. w .. e
    if key == 'k0000' then
        if g[y - 1][x - 1] == 1 then
            return 'corner/outer/se'
        end
        if g[y - 1][x + 1] == 1 then
            return 'corner/outer/sw'
        end
        if g[y + 1][x - 1] == 1 then
            return 'corner/outer/ne'
        end
        if g[y + 1][x + 1] == 1 then
            return 'corner/outer/nw'
        end
        return 'outside'
    else
        local v = BORDER_LOOKUP[key]
        if v == nil then
            return 'outside'
        else
            return v
        end
    end
end

local function generate_dungeon_room(c, room, n, s, w, e)
    local grid = {}
    for i = 1, 18 do
        local row = {}
        for j = 1, 18 do
            row[j] = 0
        end
        grid[i] = row
    end

    local room0 = 8 - room
    local room1 = 7 + room
    fill_grid(grid, room0, room0, room1, room1)
    fill_grid(grid, 8 - n, 0, 7 + n, 7)
    fill_grid(grid, 8 - s, 8, 7 + s, 15)
    fill_grid(grid, 0, 8 - w, 7, 7 + w)
    fill_grid(grid, 8, 8 - e, 15, 7 + e)

    for y = 0, 15 do
        for x = 0, 15 do
            local v = variant_from_grid(grid, x + 2, y + 2)
            place_cave_inside(c, x, y, v)
        end
    end
end


local function room_rng(plane_seed, x, y)
    return Rng.with_seed(plane_seed * 37 + x * 31 + y)
end

local DOOR_SIZES = {
    [0] = 10,
    [1] = 12,
    [2] =  7,
    [7] =  3,
}

local function door_sizes(plane_seed, x, y)
    local r = room_rng(plane_seed, x, y)
    return { r:choose_weighted(pairs(DOOR_SIZES)),
             r:choose_weighted(pairs(DOOR_SIZES)) }
end

local function fill_rect_generic(c, size, tile_map)
    if size < 0 then return end

    local c0 = 7 - size
    local c1 = 8 + size

    for y = c0 + 1, c1 - 1 do
        for x = c0 + 1, c1 - 1 do
            c:set_block(V3.new(x, y, 0), tile_map.center)
        end
    end

    for i = c0 + 1, c1 - 1 do
        c:set_block(V3.new(i, c0, 0), tile_map.edge_n)
        c:set_block(V3.new(i, c1, 0), tile_map.edge_s)
        c:set_block(V3.new(c0, i, 0), tile_map.edge_w)
        c:set_block(V3.new(c1, i, 0), tile_map.edge_e)
    end
    c:set_block(V3.new(c0, c0, 0), tile_map.corner_nw)
    c:set_block(V3.new(c1, c0, 0), tile_map.corner_ne)
    c:set_block(V3.new(c0, c1, 0), tile_map.corner_sw)
    c:set_block(V3.new(c1, c1, 0), tile_map.corner_se)
end

local function fill_rect(c, size, tileset)
    fill_rect_generic(c, size, {
        center = tileset .. '/center',
        edge_n = tileset .. '/edge/n',
        edge_s = tileset .. '/edge/s',
        edge_w = tileset .. '/edge/w',
        edge_e = tileset .. '/edge/e',
        corner_nw = tileset .. '/corner/outer/nw',
        corner_ne = tileset .. '/corner/outer/ne',
        corner_sw = tileset .. '/corner/outer/sw',
        corner_se = tileset .. '/corner/outer/se',
    })
end

local function fill_rect_inverse(c, size, tileset)
    fill_rect_generic(c, size, {
        center = 'cave_inside/center/z0',
        edge_n = tileset .. '/edge/s',
        edge_s = tileset .. '/edge/n',
        edge_w = tileset .. '/edge/e',
        edge_e = tileset .. '/edge/w',
        corner_nw = tileset .. '/corner/inner/se',
        corner_ne = tileset .. '/corner/inner/sw',
        corner_sw = tileset .. '/corner/inner/ne',
        corner_se = tileset .. '/corner/inner/nw',
    })
end

local ROOM_EXTRAS = {
    nothing = 15,
    lava = 4,
    water = 6,
    pit = 2,
    trophy = 1,
    library = 1,
    fountain = 1,
}

local ROOM_EXTRA_FUNCS = {}

function ROOM_EXTRA_FUNCS.nothing(c, size, rc)
end

function ROOM_EXTRA_FUNCS.lava(c, size, rc)
    if size < 2 then return end
    fill_rect(c, rc:gen(size - 3, size - 1), 'cave_lava')
end

function ROOM_EXTRA_FUNCS.water(c, size, rc)
    if size < 2 then return end
    fill_rect(c, rc:gen(size - 3, size - 1), 'cave_water')
end

function ROOM_EXTRA_FUNCS.pit(c, size, rc)
    if size < 2 then return end
    fill_rect(c, rc:gen(size - 3, size - 1), 'cave_pit')
end

function ROOM_EXTRA_FUNCS.trophy(c, size, rc)
    if size < 3 then return end
    if rc:gen(1, 3) <= 2 then
        local moat = rc:choose_weighted(pairs({pit = 5, lava = 2, water = 1}))
        fill_rect(c, rc:gen(2, size - 1), 'cave_' .. moat)
        fill_rect_inverse(c, 1, 'cave_' .. moat)
    end
    c:add_structure(V3.new(rc:gen(7, 8), 7, 0), 'trophy')
end

local function maybe_place_bookshelf(c, x, y, rc)
    if rc:gen(1, 10) <= 3 then return end
    local variant = rc:choose_weighted(pairs({[0] = 7, [1] = 2, [2] = 1}))
    c:add_structure(V3.new(x, y, 0), 'bookshelf/' .. variant)
end

function ROOM_EXTRA_FUNCS.library(c, size, rc)
    if size < 2 then return end
    for dy = 2, size, 2 do
        for dx = 2, size do
            maybe_place_bookshelf(c, 8 - dx, 8 - dy, rc)
            maybe_place_bookshelf(c, 8 - dx, 7 + dy, rc)
            maybe_place_bookshelf(c, 7 + dx, 8 - dy, rc)
            maybe_place_bookshelf(c, 7 + dx, 7 + dy, rc)
        end
    end
end

function ROOM_EXTRA_FUNCS.fountain(c, size, rc)
    if size < 2 then return end
    c:add_structure(V3.new(7, 7, 0), 'fountain')
end



local function generate_dungeon(c, cpos, rp, rc)
    local plane_seed = rp:gen(0, 0x3fffffff)

    local cx = cpos:x()
    local cy = cpos:y()

    local ds_here = door_sizes(plane_seed, cx, cy)
    local ds_north = door_sizes(plane_seed, cx, cy - 1)
    local ds_west = door_sizes(plane_seed, cx - 1, cy)

    if cx == 0 and cy == 0 then
        generate_dungeon_room(c, 4, ds_north[1], ds_here[1], ds_west[2], ds_here[2])
        c:add_structure(V3.new(7, 7, 0), 'dungeon_exit')
    else
        local room_size = rc:gen(1, 7)

        generate_dungeon_room(c, room_size, ds_north[1], ds_here[1], ds_west[2], ds_here[2])

        local extra = rc:choose_weighted(pairs(ROOM_EXTRAS))
        ROOM_EXTRA_FUNCS[extra](c, room_size, rc)
    end
end


function outpost_ffi.callbacks.generate_chunk(c, plane_name, cpos, plane_rng, chunk_rng)
    local r = chunk_rng

    if plane_name == 'Everfree Forest' then
        generate_forest(c, cpos, chunk_rng)
    else if plane_name == 'Dungeon' then
        generate_dungeon(c, cpos, plane_rng, chunk_rng)
    end end
end



function outpost_ffi.callbacks.apply_structure_extra(s, k, v)
    if k == 'loot' then
        print('spawning loot: ', v)
        if v:sub(1, 2) == 'H ' then
            s:inventory('contents'):update('hat', 1)
            v = v:sub(3)
        end

        local space = v:find(' ')
        local count = v:sub(1, space - 1)
        local item = v:sub(space + 1)
        s:inventory('contents'):update(item, 0 + count)
    end
end



