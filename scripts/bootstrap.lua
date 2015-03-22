-- Override print to output to stderr.  stdout is used for communication with
-- the server wrapper.
function print(...)
    s = ''
    for i = 1, select('#', ...) do
        x = select(i, ...)
        s = s .. tostring(x) .. '\t'
    end
    io.stderr:write(s .. '\n')
end

local function dump_rec(x, n)
    for k,v in pairs(x) do
        if type(v) == 'table' then
            print(n .. tostring(k) .. ':')
            dump_rec(v, n .. '  ')
        else
            print(n .. tostring(k) .. ': ' .. tostring(v))
        end
    end
end

local function dump(x)
    if type(x) == 'table' then
        dump_rec(x, '')
    else
        print(x)
    end
end

package.loaded.bootstrap = {
    dump = dump,
}


-- Put some type tables in global scope
V3 = outpost_ffi.types.V3.table
V2 = outpost_ffi.types.V2.table
World = outpost_ffi.types.World.table

Rng = outpost_ffi.types.Rng.table
GenChunk = outpost_ffi.types.GenChunk.table
ValuesMut = outpost_ffi.types.ValuesMut.table

ConstantField = outpost_ffi.types.ConstantField.table
RandomField = outpost_ffi.types.RandomField.table
FilterField = outpost_ffi.types.FilterField.table
BorderField = outpost_ffi.types.BorderField.table
DiamondSquare = outpost_ffi.types.DiamondSquare.table

IsoDiskSampler = outpost_ffi.types.IsoDiskSampler.table


require('outpost.userdata')
require('outpost.extra')
require('outpost.eval')
local action = require('outpost.action')
local command = require('outpost.command')

require('inventory')
local tools = require('tools')
require('structure_items')
require('chest')
require('anvil')
require('ward_item')
local ward = require('ward')
require('mallet')


function action.open_inventory(c)
    c:open_inventory(c:pawn():inventory('main'))
end


-- 'tree' behavior
function action.use.tree(c, s)
    local count = c:pawn():inventory('main'):update('wood', 2)
end

function tools.handler.axe.tree(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:replace('stump')
    inv:update('wood', 15)
end

function tools.handler.axe.stump(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:destroy()
    inv:update('wood', 5)
end


-- 'rock' behavior
function action.use.rock(c, s)
    local count = c:pawn():inventory('main'):update('stone', 2)
end

function tools.handler.pick.rock(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:destroy()
    inv:update('stone', 20)
    print(math.random())
    if math.random() < 0.2 then
        print(inv:update('crystal', 1))
    end
end


-- Commands
function command.handler.where(client, args)
    local pos = client:pawn():pos()
    local x = pos:x()
    local y = pos:y()
    client:send_message('Location: ' .. tostring(x) .. ', ' .. tostring(y))
end

local spawn_point = V3.new(32, 32, 0)

function command.handler.spawn(client, args)
    client:pawn():teleport(spawn_point)
end

function command.handler.sethome(client, args)
    local home = client:pawn():pos()
    client:extra().home_pos = home
    client:send_message('Set home to ' .. tostring(home))
end

function command.handler.home(client, args)
    local home = client:extra().home_pos or spawn_point
    client:pawn():teleport(home)
end


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

    for y = 0, 15 do
        for x = 0, 15 do
            local border = water_border[y * 16 + x + 1]
            if border ~= 0 then
                c:set_block(V3.new(x, y, 0), 'cave/' .. TILE_ID_MAP[border + 1] .. '/z0')
                c:set_block(V3.new(x, y, 1), 'cave/' .. TILE_ID_MAP[border + 1] .. '/z1')
            else
                c:set_block(V3.new(x, y, 0), r:choose_weighted(pairs(grass)))
            end
        end
    end

    local structures = {
        ['tree'] = 2,
        ['rock'] = 1,
    }
    local p = sampler:get_points(min, max)

    for i = 1, #p do
        local wb = water:get_region(p[i], p[i] + V2.new(4, 2))
        local ok = true
        for j = 1, #wb do
            if wb[j] ~= 0 then
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


print('\n\nup and running')
