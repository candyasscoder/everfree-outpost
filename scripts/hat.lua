local action = require('outpost.action')

function action.use_item.hat(c, inv)
    inv:update('hat', -1)
    c:pawn():inventory('equipped'):update('hat', 1)
    c:pawn():update_appearance(0x100, 0x100)
    c:send_message('Press S to unequip')
end

function action.use_ability._(c, inv)
    if c:pawn():inventory('equipped'):count('hat') == 0 then
        return
    end

    c:pawn():inventory('equipped'):update('hat', -1)
    c:pawn():inventory('main'):update('hat', 1)
    c:pawn():update_appearance(0x100, 0)
end
