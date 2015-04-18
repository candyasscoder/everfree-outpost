var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');

/** @constructor */
function MusicTest() {
    this.dom = util.fromTemplate('music-test', {});
    this.keys = widget.NULL_KEY_HANDLER;
    var select = this.dom.getElementsByClassName('music-select')[0];
    var file = this.dom.getElementsByClassName('music-file')[0];
    var player = this.dom.getElementsByClassName('music-player')[0];

    var objectUrl = null;

    function unload() {
        if (objectUrl != null) {
            window.URL.revokeObjectURL(objectUrl);
        }
    }

    select.onchange = function() {
        unload();
        if (select.value == 'none') {
            player.src = null;
            player.load();
        } else {
            player.src = select.value;
            player.play();
        }
    };

    file.onchange = function() {
        unload();
        player.src = window.URL.createObjectURL(file.files[0]);
        player.play();
    };

    this.player = player;
    document.body.appendChild(this.player);
}
exports.MusicTest = MusicTest;

MusicTest.prototype.handleOpen = function(dialog) {
    this.player.controls = true;
    this.dom.appendChild(this.player);
    if (this.player.src != null) {
        this.player.play();
    }
};

MusicTest.prototype.handleClose = function(dialog) {
    this.player.controls = false;
    document.body.appendChild(this.player);
    if (this.player.src != null) {
        this.player.play();
    }
};
