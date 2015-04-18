var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');

var TRACKS = [
    ];

/** @constructor */
function MusicTest() {
    this.dom = util.fromTemplate('music-test', {});
    this.keys = widget.NULL_KEY_HANDLER;
    var select = this.dom.getElementsByClassName('music-select')[0];
    var file = this.dom.getElementsByClassName('music-file')[0];
    var player = this.dom.getElementsByClassName('music-player')[0];

    var objectUrl = null;

    util.element('option', ['value=none', 'text=None'], select);
    var opt_custom = util.element('option',
            ['value=custom', 'text=Custom', 'disabled=true'], select);

    function makeUrl(blob) {
        if (objectUrl != null) {
            window.URL.revokeObjectURL(objectUrl);
        }
        objectUrl = window.URL.createObjectURL(blob);
        opt_custom.disabled = false;
        return objectUrl;
    }

    select.onchange = function() {
        if (select.value == 'none') {
            player.src = '';
            player.load();
        } else if (select.value == 'custom') {
            player.src = objectUrl;
            player.load();
        } else {
            player.src = select.value;
            player.play();
        }
    };

    file.onchange = function() {
        player.src = makeUrl(file.files[0]);
        player.play();
        select.value = 'custom';
        opt_custom.textContent = file.files[0].name;
    };

    for (var i = 0; i < TRACKS.length; ++i) {
        var name = TRACKS[i];

        var option = util.element('option', [
                'value=music/' + name,
                'text=' + name], select);
    }

    this.player = player;
    document.body.appendChild(this.player);
}
exports.MusicTest = MusicTest;

MusicTest.prototype.handleOpen = function(dialog) {
    this.player.controls = true;
    this.dom.appendChild(this.player);
    if (this.player.src != '') {
        this.player.play();
    }
};

MusicTest.prototype.handleClose = function(dialog) {
    this.player.controls = false;
    document.body.appendChild(this.player);
    if (this.player.src != '') {
        this.player.play();
    }
};
