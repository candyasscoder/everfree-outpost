local action = require('outpost.action')
local util = require('outpost.util')
local structure_items = require('lib.structure_items')


-- 0: no floor, 1: floor on -x/-y half, 2: floor on +x/+y half, 3: both
-- Edge order is N, S, W, E
local FLOOR_EDGES = {
    ['center/v0'] =         {3, 3, 3, 3},
    ['edge/n'] =            {0, 3, 2, 2},
    ['edge/s'] =            {3, 0, 1, 1},
    ['edge/w'] =            {2, 2, 0, 3},
    ['edge/e'] =            {1, 1, 3, 0},
    ['corner/outer/nw'] =   {0, 2, 0, 2},
    ['corner/outer/sw'] =   {2, 0, 0, 1},
    ['corner/outer/ne'] =   {0, 1, 2, 0},
    ['corner/outer/se'] =   {1, 0, 1, 0},
    ['corner/inner/nw'] =   {2, 3, 2, 3},
    ['corner/inner/ne'] =   {1, 3, 3, 2},
    ['corner/inner/se'] =   {3, 1, 3, 1},
    ['corner/inner/sw'] =   {3, 2, 1, 3},
}

local FLOOR_ORDER = {
    'center/v0',
    'edge/n',
    'edge/s',
    'edge/w',
    'edge/e',
    'corner/outer/nw',
    'corner/outer/sw',
    'corner/outer/ne',
    'corner/outer/se',
    'corner/inner/nw',
    'corner/inner/ne',
    'corner/inner/se',
    'corner/inner/sw',
}

local FLOOR_INFO = { edges = FLOOR_EDGES, order = FLOOR_ORDER, layer = 0 }


local INFO = {}


local SIDE_N = 1
local SIDE_S = 2
local SIDE_W = 3
local SIDE_E = 4

local SIDE_DIR = {
    V3.new(0, -1, 0),
    V3.new(0,  1, 0),
    V3.new(-1, 0, 0),
    V3.new( 1, 0, 0),
}

local SIDE_FLIP = {
    [SIDE_N] = SIDE_S,
    [SIDE_S] = SIDE_N,
    [SIDE_W] = SIDE_E,
    [SIDE_E] = SIDE_W,
}


local function get_edge(kind, plane, pos, side)
    local info = INFO[kind]

    local s_pos = pos + SIDE_DIR[side]
    local s = World.get():find_structure_at_point_layer(plane, s_pos, info.layer)
    if s == nil then
        return 0
    end

    local t = s:template()
    if not t:startswith(kind .. '/') then
        return 0
    end

    local variant = t:sub(#kind + 2)

    local x = info.edges
    if x == nil then return 0 end
    x = x[variant]
    if x == nil then return 0 end
    return x[SIDE_FLIP[side]]
end

local function calc_similarity(a, b)
    local sim = 0
    for i = 1, 4 do
        if a[i] == b[i] then
            -- Value matching up non-empty sides slightly more than matching up
            -- empty sides.  This makes placing a floor on an inner corner
            -- prefer the actual inner corner tile over an edge tile.
            -- (Corner matches 3, 3, 1/2, and mismatches 0; edge matches 3, 0,
            -- 1/2, and mismatches 3.)
            if a[i] == 0 then
                sim = sim + 10
            else
                sim = sim + 11
            end
        end
    end
    return sim
end

local function floor_has_interesting_edge(edges)
    for i = 1, 4 do
        local e = edges[i]
        if e == 1 or e == 2 then
            return true
        end
    end
    return false
end

local function choose_floor_rotation(kind, plane, pos)
    local info = INFO[kind]

    local edges = {
        get_edge(kind, plane, pos, SIDE_N),
        get_edge(kind, plane, pos, SIDE_S),
        get_edge(kind, plane, pos, SIDE_W),
        get_edge(kind, plane, pos, SIDE_E),
    }

    if not floor_has_interesting_edge(edges) then
        return kind .. '/' .. info.order[1]
    end

    local best_i = 0
    local best_sim = 0
    for i,v in ipairs(info.order) do
        local sim = calc_similarity(edges, info.edges[v])
        if sim > best_sim then
            best_sim = sim
            best_i = i
        end
    end

    return kind .. '/' .. info.order[best_i]
end


local function add_floor_item(item_name, template_base)
    if template_base == nil then
        template_base = item_name
    end

    INFO[template_base] = FLOOR_INFO

    action.use_item[item_name] = function(c, inv)
        local pawn = c:pawn()
        local plane = pawn:plane()
        local pos = util.hit_tile(pawn)

        local template = choose_floor_rotation(template_base, plane, pos)
        structure_items.use_item(c, inv, item_name, template)
    end

    local function interact(c, s)
        structure_items.use_structure(c, s, item_name)
    end
    for _, v in ipairs(INFO[template_base].order) do
        action.use[template_base .. '/' .. v] = interact
    end
end


return {
    add_floor_item = add_floor_item,
}
