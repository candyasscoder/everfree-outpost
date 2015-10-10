local action = require('core.action')

function make_hat(name, id)
    action.use_item[name] = function(c, inv)
        if c:pawn():inventory('ability'):count('ability/remove_hat') > 0 then
            return
        end

        inv:update(name, -1)
        c:pawn():extra().hat_type = name
        c:pawn():inventory('ability'):update('ability/remove_hat', 1)
        c:pawn():update_appearance(0x3c0000, id * 0x040000)
    end
end

make_hat('hat', 1)
make_hat('party_hat', 2)

function action.use_ability.remove_hat(c, inv)
    c:pawn():inventory('ability'):update('ability/remove_hat', -1)
    c:pawn():inventory('main'):update(c:pawn():extra().hat_type, 1)
    c:pawn():extra().hat_type = nil
    c:pawn():update_appearance(0x3c0000, 0x000000)
end
