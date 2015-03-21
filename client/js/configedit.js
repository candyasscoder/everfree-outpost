var Config = require('config').Config;
var ConfigEditor = require('ui/configedit').ConfigEditor;

function $(x) { return document.getElementById(x); }

function init() {
    $('reset-login').addEventListener('click', function() {
        $('confirm-reset-login').disabled = false;
    });
    $('confirm-reset-login').disabled = true;

    $('confirm-reset-login').addEventListener('click', function() {
        Config.login_name.reset();
        Config.login_secret.reset();
    });

    $('enable-motion-prediction').addEventListener('change', function() {
        var value = $('enable-motion-prediction').checked;
        Config.motion_prediction.set(value);
    });
    $('enable-motion-prediction').checked = Config.motion_prediction.get();

    $('open-editor').addEventListener('click', function() {
        $('open-editor').disabled = true;
        var editor = new ConfigEditor();
        document.body.appendChild(editor.dom);
    });
    // Firefox saves the 'disabled' setting across refresh.
    $('open-editor').disabled = false;
}

document.addEventListener('DOMContentLoaded', init);
