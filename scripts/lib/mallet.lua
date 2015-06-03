local action = require('outpost.action')
local util = require('outpost.util')
local ward = require('lib.ward')

local replacements = {}

local function use_mallet(c, inv)
    local s = util.hit_structure(c:pawn())
    if s == nil then
        return
    end

    if not ward.check(c, s:pos()) then
        return
    end

    new_template = replacements[s:template()]
    if new_template ~= nil then
        s:replace(new_template)
    end
end

local function mallet_cycle(base, xs)
    for i = 1, #xs - 1 do
        local t_cur = base .. xs[i]
        local t_next = base .. xs[i + 1]
        replacements[t_cur] = t_next
        if action.use[t_next] == nil then
            action.use[t_next] = action.use[t_cur]
        end
    end
    replacements[base .. xs[#xs]] = base .. xs[1]
end

return {
    replacements = replacements,
    use_mallet = use_mallet,
    mallet_cycle = mallet_cycle,
}
