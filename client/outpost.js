(function() {

var $ = document.getElementById.bind(document);


var canvas = document.createElement('canvas');
document.body.appendChild(canvas);
canvas.width = canvas.clientWidth;
canvas.height = canvas.clientHeight;

var ctx = canvas.getContext('2d');

var animating = false;

function frameWrapper() {
    frame();
    if (animating) {
        window.requestAnimationFrame(frameWrapper);
    }
}

function startAnimation() {
    animating = true;
    window.requestAnimationFrame(frameWrapper);
}

function stopAnimation() {
    animating = false;
}


function Sheet(image, item_width, item_height) {
    this.image = image;
    this.item_width = item_width;
    this.item_height = item_height;
}

Sheet.prototype = {
    'drawInto': function(ctx, i, j, x, y) {
        ctx.drawImage(this.image,
                j * this.item_width,
                i * this.item_height,
                this.item_width,
                this.item_height,
                x,
                y,
                this.item_width,
                this.item_height);
    },
};


function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}

AssetLoader.prototype = {
    'addImage': function(name, url) {
        var img = new Image();

        var this_ = this;
        img.onload = function() { this_._handleAssetLoad(); };

        img.src = url;
        this._addPendingAsset(name, img);
    },

    '_addPendingAsset': function(name, asset) {
        this.assets[name] = asset;
        this.pending += 1;
    },

    '_handleAssetLoad': function() {
        this.pending -= 1;
        this.loaded += 1;
        if (typeof this.onprogress == 'function') {
            this.onprogress(this.loaded / (this.pending + this.loaded));
        }
        if (this.pending == 0 && typeof this.onload == 'function') {
            this.onload();
        }
    },
};



var loader = new AssetLoader();
loader.addImage('pony_base_f', 'assets/maresprite.png');
var assets = loader.assets;

var sprite_sheet = new Sheet(assets.pony_base_f, 96, 96);

ctx.globalCompositeOperation = 'copy';

var start_time = Date.now();
loader.onload = function() {
    document.body.removeChild($('banner-bg'));
    start_time = Date.now();
    startAnimation();
    console.log(this.loaded, 'assets loaded');
};

function frame() {
    var delta = Date.now() - start_time;
    var frame = Math.floor(delta / 100) % 6;
    sprite_sheet.drawInto(ctx, 3, 12 + frame, 100, 100);
}

console.log('startup done');

})();
