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
        c:add_structure_with_extras(V3.new(x, y - 3, 0), 'chest', { loot = loot })
    end
end

local function choose_loot(r)
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

function outpost_ffi.callbacks.generate_chunk(c, cpos, r)
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
