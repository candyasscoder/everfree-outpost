local WARD_RADIUS = 16
local WARD_SPACING = 48

local function ward_info_table()
    local info = World.get():extra().ward_info
    if info ~= nil then
        return info
    else
        info = {
            server = { pos = V3.new(0, 0, 0), name = 'the server' },
        }
        World.get():extra().ward_info = info
        return info
    end
end

local function ward_info(owner)
    print('get', owner:id())
    return ward_info_table()[owner:id()]
end

local function set_ward_info(owner, info)
    ward_info_table()[owner:id()] = info
end


local function ward_perm_table()
    local perm = World.get():extra().ward_perm
    if perm ~= nil then
        return perm
    else
        perm = {}
        World.get():extra().ward_perm = perm
        return perm
    end
end

local function permit(owner, name)
    local perm = ward_perm_table()
    local id = owner:id()
    if perm[id] == nil then
        perm[id] = {}
    end
    perm[id][name] = true
end

local function revoke(owner, name)
    local perm = ward_perm_table()
    local id = owner:id()
    if perm[id] == nil or perm[id][name] == nil then
        return false
    end
    perm[id][name] = nil
    return true
end

local function check_perm(owner, name)
    if owner == nil then
        -- This is the case for the built-in spawn ward..
        return false
    end
    local perm = ward_perm_table()
    local id = owner:id()
    if perm[id] == nil then
        return false
    end
    if perm[id][name] then
        return true
    else
        return false
    end
end


local function add_ward(c, s)
    local owner = c:stable_id()
    s:extra().owner = owner
    set_ward_info(owner, {
        pos = s:pos(),
        name = c:name(),
        owner = owner,
    })
end

local function remove_ward(c)
    set_ward_info(c:stable_id(), nil)
end

local function find_ward(c, pos, radius)
    -- TODO: build an index for this lookup
    local stable_id = c:stable_id():id()
    for owner, info in pairs(ward_info_table()) do
        if owner ~= stable_id then
            local dist = (info.pos - pos):abs():max()
            if dist <= radius then
                return info, dist
            end
        end
    end
    return nil
end

return {
    WARD_RADIUS = WARD_RADIUS,
    WARD_SPACING = WARD_SPACING,

    add_ward = add_ward,
    remove_ward = remove_ward,
    find_ward = find_ward,
    ward_info = ward_info,

    permit = function(c, name) permit(c:stable_id(), name) end,
    revoke = function(c, name) return revoke(c:stable_id(), name) end,

    check = function(c, pos)
        -- There are no wards outside the forest.
        if c:pawn():plane():name() ~= PLANE_FOREST then
            return true
        end

        local info, dist = find_ward(c, pos, WARD_RADIUS)
        if info ~= nil then
            if check_perm(info.owner, c:name()) then
                return true
            end
            c:send_message('This area belongs to ' .. info.name)
            if c:extra().superuser then
                return true
            else
                return false
            end
        else
            return true
        end
    end,
}
