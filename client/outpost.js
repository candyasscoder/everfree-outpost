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
loader.addImage('pony_f_base', 'assets/maresprite.png');
loader.addImage('pony_f_eyes_blue', 'assets/type1blue.png');
loader.addImage('pony_f_horn', 'assets/marehorn.png');
loader.addImage('pony_f_wing_front', 'assets/frontwingmare.png');
loader.addImage('pony_f_wing_back', 'assets/backwingmare.png');
loader.addImage('pony_f_mane_1', 'assets/maremane1.png');
loader.addImage('pony_f_tail_1', 'assets/maretail1.png');
var assets = loader.assets;

var sheet_f_base = new Sheet(assets.pony_f_base, 96, 96);
var sheet_f_eyes_blue = new Sheet(assets.pony_f_eyes_blue, 96, 96);
var sheet_f_horn = new Sheet(assets.pony_f_horn, 96, 96);
var sheet_f_wing_front = new Sheet(assets.pony_f_wing_front, 96, 96);
var sheet_f_wing_back = new Sheet(assets.pony_f_wing_back, 96, 96);
var sheet_f_mane_1 = new Sheet(assets.pony_f_mane_1, 96, 96);
var sheet_f_tail_1 = new Sheet(assets.pony_f_tail_1, 96, 96);

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
    ctx.globalCompositeOperation = 'copy';
    sheet_f_base.drawInto(ctx, 3, 12 + frame, 100, 100);
    ctx.globalCompositeOperation = 'source-over';
    sheet_f_eyes_blue.drawInto(ctx, 3, 12 + frame, 100, 100);
    sheet_f_wing_front.drawInto(ctx, 3, 12 + frame, 100, 100);
    sheet_f_horn.drawInto(ctx, 3, 12 + frame, 100, 100);
    sheet_f_tail_1.drawInto(ctx, 3, 12 + frame, 100, 100);
    sheet_f_mane_1.drawInto(ctx, 3, 12 + frame, 100, 100);
}

console.log('startup done');

})();
