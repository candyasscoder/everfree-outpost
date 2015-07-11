local string = require('string')

function string.startswith(s, prefix)
    return s:sub(1, #prefix) == prefix
end
