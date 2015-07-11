local action = require('core.action')
local mallet = require('outpost.lib.mallet')
local tools = require('outpost.lib.tools')

action.use_item.mallet = mallet.use_mallet
action.use_item.axe = tools.mk_use_tool('axe')
action.use_item.pick = tools.mk_use_tool('pick')
