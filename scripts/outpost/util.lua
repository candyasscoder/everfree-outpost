local function hit_tile(entity)
    local pos = entity:pos()
    -- TODO: hardcoded constants based on entity size and tile size
    local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
    return target:pixel_to_tile()
end

local function hit_structure(entity)
    return entity:world():find_structure_at_point(entity:plane(), hit_tile(entity))
end

return {
    hit_tile = hit_tile,
    hit_structure = hit_structure,
}
