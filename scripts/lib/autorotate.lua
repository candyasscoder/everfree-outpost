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

local function find_best_variant_filtered(info, target, filter, calc_similarity)
    local best_i = 0
    local best_sim = 0
    for i,v in ipairs(info.order) do
        if filter(v) then
            local sim = calc_similarity(target, info.edges[v])
            if sim > best_sim then
                best_sim = sim
                best_i = i
            end
        end
    end
    return info.order[best_i]
end

local function find_best_variant(info, target, calc_similarity)
    return find_best_variant_filtered(info, target, function(v) return true end, calc_similarity)
end

local function find_template_base(name)
    local cur = 1
    while true do
        local slash = name:find('/', cur, true)
        if slash == nil then
            return nil
        end
        cur = slash

        local base = name:sub(1, slash - 1)
        if INFO[base] ~= nil then
            return base
        end
    end
end


local function calc_floor_similarity(a, b)
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

    local variant = find_best_variant(info, edges, calc_floor_similarity)
    if variant == nil then
        variant = info.order[1]
    end
    return kind .. '/' .. variant
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



-- 0: nothing, 1: horizontal (shows inside), 2: horizontal (outside), 3: vertical
-- Edge order is N, S, W, E
local HOUSE_WALL_EDGES = {
    ['edge/horiz/in'] =     {0, 0, 1, 1},
    ['edge/horiz/out'] =    {0, 0, 2, 2},
    ['edge/vert'] =         {3, 3, 0, 0},
    ['corner/nw/in'] =      {0, 3, 0, 1},
    ['corner/ne/in'] =      {0, 3, 1, 0},
    ['corner/sw/in'] =      {3, 0, 0, 1},
    ['corner/se/in'] =      {3, 0, 1, 0},
    ['corner/nw/out'] =     {0, 3, 0, 2},
    ['corner/ne/out'] =     {0, 3, 2, 0},
    ['corner/sw/out'] =     {3, 0, 0, 2},
    ['corner/se/out'] =     {3, 0, 2, 0},
    ['tee/n/in'] =          {3, 0, 1, 1},
    ['tee/n/out'] =         {3, 0, 2, 2},
    ['tee/e/in'] =          {3, 3, 0, 1},
    ['tee/e/out'] =         {3, 3, 0, 2},
    ['tee/s/in_in'] =       {0, 3, 1, 1},
    ['tee/s/in_out'] =      {0, 3, 1, 2},
    ['tee/s/out_out'] =     {0, 3, 2, 2},
    ['tee/s/out_in'] =      {0, 3, 2, 1},
    ['tee/w/in'] =          {3, 3, 1, 0},
    ['tee/w/out'] =         {3, 3, 2, 0},
    ['cross/in_in'] =       {3, 3, 1, 1},
    ['cross/in_out'] =      {3, 3, 1, 2},
    ['cross/out_out'] =     {3, 3, 2, 2},
    ['cross/out_in'] =      {3, 3, 2, 1},
    ['door/in/closed'] =    {0, 0, 1, 1},
    ['door/out/closed'] =   {0, 0, 2, 2},
}

local HOUSE_WALL_ORDER = {
    'edge/horiz/in',
    'edge/horiz/out',
    'edge/vert',
    'corner/nw/in',
    'corner/ne/in',
    'corner/sw/in',
    'corner/se/in',
    'corner/nw/out',
    'corner/ne/out',
    'corner/sw/out',
    'corner/se/out',
    'tee/n/in',
    'tee/n/out',
    'tee/e/in',
    'tee/e/out',
    'tee/s/in_in',
    'tee/s/in_out',
    'tee/s/out_out',
    'tee/s/out_in',
    'tee/w/in',
    'tee/w/out',
    'cross/in_in',
    'cross/in_out',
    'cross/out_out',
    'cross/out_in',
    'door/in/closed',
    'door/out/closed',
}

local HOUSE_WALL_INFO = {
    edges = HOUSE_WALL_EDGES,
    order = HOUSE_WALL_ORDER,
    layer = 1,
}


local function calc_house_similarity(a, b)
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

local function choose_house_wall_rotation(kind, variant_base, plane, pos)
    local info = INFO[kind]

    if variant_base[#variant_base] ~= '/' then
        variant_base = variant_base .. '/'
    end

    local function filter(v)
        return v:startswith(variant_base)
    end


    local s_floor = World.get():find_structure_at_point_layer(plane, pos, 0)
    if s_floor ~= nil then
        local floor_base = find_template_base(s_floor:template())
        local floor_variant = s_floor:template():sub(#floor_base + 2)
        if floor_variant ~= 'center/v0' then
            local floor_edges = INFO[floor_base].edges[floor_variant]

            local best = find_best_variant_filtered(info, floor_edges, filter, function(f, w)
                local sim = 0
                for i = 1, 4 do
                    local ok = false
                    if w[i] == 0 then
                        ok = f[i] == 0 or f[i] == 3
                    else if w[i] == 1 then
                        ok = f[i] == 2 or f[i] == 3
                    else if w[i] == 2 then
                        ok = f[i] == 1
                    else if w[i] == 3 then
                        ok = f[i] ~= 0
                    end end end end
                    if ok then
                        sim = sim + 1
                    end
                end
                return sim
            end)

            if best ~= nil then
                return kind .. '/' .. best
            end
        end
    end


    local edges = {
        get_edge(kind, plane, pos, SIDE_N),
        get_edge(kind, plane, pos, SIDE_S),
        get_edge(kind, plane, pos, SIDE_W),
        get_edge(kind, plane, pos, SIDE_E),
    }

    local variant = find_best_variant_filtered(info, edges, filter, function(a, b)
        local sim = 0
        for i = 1, 4 do
            if a[i] == b[i] then
                sim = sim + 1
            end
        end
        return sim
    end)
    if variant == nil then
        variant = info.order[1]
    end
    return kind .. '/' .. variant
end

local function add_house_wall_item(item_name, template_base, variant_base)
    if template_base == nil then
        template_base = item_name
    end

    INFO[template_base] = HOUSE_WALL_INFO

    action.use_item[item_name] = function(c, inv)
        local pawn = c:pawn()
        local plane = pawn:plane()
        local pos = util.hit_tile(pawn)

        local template = choose_house_wall_rotation(template_base, variant_base, plane, pos)
        structure_items.use_item(c, inv, item_name, template)
    end

    local function interact(c, s)
        structure_items.use_structure(c, s, item_name)
    end
    for _, v in ipairs(INFO[template_base].order) do
        if v:startswith(variant_base .. '/') then
            action.use[template_base .. '/' .. v] = interact
        end
    end
end



return {
    add_floor_item = add_floor_item,
    add_house_wall_item = add_house_wall_item,
}
