local action = require('core.action')
local util = require('core.util')

local COLORS = { 'red', 'orange', 'yellow', 'green', 'blue', 'purple' }

local ITEM_PREFIX = 'gem/'
local NORMAL_PREFIX = 'dungeon/gem_slot/normal/'
local FIXED_PREFIX = 'dungeon/gem_slot/fixed/'

local function drop(prefix, name)
    return name:sub(#prefix + 1)
end

local NORMAL_EMPTY = NORMAL_PREFIX .. 'empty'


local function handle_use_fixed(c, s)
    c:send_message("You can't remove this gem.")
end

local function handle_use_normal(c, s)
    local color = drop(NORMAL_PREFIX, s:template())
    local item_name = ITEM_PREFIX .. color

    local inv = c:pawn():inventory('main')
    if inv:count(item_name) == 255 then
        return
    end
    inv:update(item_name, 1)
    s:replace(NORMAL_EMPTY)

    local pid = s:extra().puzzle_id
    local p = s:plane():extra().puzzles[pid]
    update_puzzle(p, s:extra().slot_index, 'empty')
end

local function handle_use_item(c, inv, color)
    local s = util.hit_structure(c:pawn())
    if s == nil or s:template() ~= NORMAL_EMPTY then
        return
    end

    inv:update(ITEM_PREFIX .. color, -1)
    s:replace(NORMAL_PREFIX .. color)

    local pid = s:extra().puzzle_id
    local p = s:plane():extra().puzzles[pid]
    update_puzzle(p, s:extra().slot_index, color)
end


local COLOR_VAL = {
    red = 0,
    orange = 1,
    yellow = 2,
    green = 3,
    blue = 4,
    purple = 5,
    empty = -1,
}

local function update_puzzle(p, slot, color)
    p.slots[slot] = COLOR_VAL[color]

    if puzzle_solved(p) then
        if not p.door_open then
            door.open(p.door:get())
            p.door_open = true
        end
    else
        if p.door_open then
            door.close(p.door:get())
            p.door_open = false
        end
    end
end

local function puzzle_solved(p)
    for i in 1 .. #p.slots do
        local c = p.slots[i]
        if c < 0 then
            return false
        end

        -- Slots 1/3/... should contain colors 0 (red), 2 (yellow), or 4 (blue)
        if i % 2 ~= (c + 1) % 2 then
            return false
        end

        -- Slots 2/4/... (which contain secondary colors) should match the
        -- primaries on either side.
        -- TODO: currently assumes #p.slots is odd
        if i % 2 == 0 then
            local a = (c - 1 + 6) % 6
            local b = (c + 1) % 6

            local l = p.slots[i - 1]
            local r = p.slots[i + 1]

            if not ((l == a and r == b) or (l == b and r == a)) then
                return false
            end
        end
    end
    return true
end


local function get_puzzle(plane, pid)
    local puzzles = plane:extra().puzzles
    if puzzles == nil then
        puzzles = {}
        plane:extra().puzzles = puzzles
    end

    local p = puzzles[pid]
    if p == nil then
        p = {
            door = nil,
            door_open = false,
            slots = {},
        }
        puzzles[pid] = p
    end

    return p
end

local function apply_gem_puzzle_slot(s, k, v)
    local pid, slot, init = v:match('(.*),(.*),(.*)')
    local p = get_puzzle(s:plane(), pid)
    p.slots[slot] = COLOR_VAL[init]
    s:extra().puzzle_id = pid
end

local function apply_gem_puzzle_door(s, k, v)
    local pid = v
    local p = get_puzzle(s:plane(), pid)
    p.door = s:stable_id()
    s:extra().puzzle_id = pid
end


for i, color in ipairs(COLORS) do
    action.use[FIXED_PREFIX .. color] = handle_use_fixed
    action.use[NORMAL_PREFIX .. color] = handle_use_normal
    action.use_item[ITEM_PREFIX .. color] = function(c, inv)
        handle_use_item(c, inv, color)
    end
end


return {
    apply_gem_puzzle_slot = apply_gem_puzzle_slot,
    apply_gem_puzzle_door = apply_gem_puzzle_door,
}
