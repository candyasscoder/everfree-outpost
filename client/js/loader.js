/** @constructor */
function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}
exports.AssetLoader = AssetLoader;

AssetLoader.prototype.addImage = function(name, url, callback) {
    var img = new Image();

    var this_ = this;
    img.onload = function() {
        if (callback != null) {
            callback(img);
        }
        this_._handleAssetLoad();
    };

    img.src = url;
    this._addPendingAsset(name, img);
};

AssetLoader.prototype.addJson = function(name, url, callback) {
    var xhr = new XMLHttpRequest();
    xhr.open('GET', url, true);

    xhr.responseType = 'json';

    var this_ = this;
    xhr.onreadystatechange = function() {
        if (this.readyState == XMLHttpRequest.DONE) {
            if (callback != null) {
                callback(this.response);
            }
            this_._handleAssetLoad();
        }
    };

    xhr.send();
    this._addPendingAsset(name, xhr);
};

AssetLoader.prototype._addPendingAsset = function(name, asset) {
    if (name != null) {
        this.assets[name] = asset;
    }
    this.pending += 1;
    this._handleProgress();
};

AssetLoader.prototype._handleAssetLoad = function() {
    this.pending -= 1;
    this.loaded += 1;
    this._handleProgress();
    if (this.pending == 0 && typeof this.onload == 'function') {
        this.onload();
    }
};

AssetLoader.prototype._handleProgress = function() {
    if (typeof this.onprogress == 'function') {
        this.onprogress(this.loaded, this.pending + this.loaded);
    }
};
