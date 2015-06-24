/** @constructor */
function Banner() {
    this.container = document.getElementById('banner-bg');
    this.text = document.getElementById('banner-text');
    this.bar = document.getElementById('banner-bar');

    this._keyboard = null;
}
exports.Banner = Banner;

Banner.prototype.hide = function() {
    if (this._keyboard != null) {
        this._keyboard.popHandler();
        this._keyboard = null;
    }

    this.container.classList.add('hidden');
};

Banner.prototype.show = function(text, fill_amount, keyboard, handler) {
    if (keyboard != null) {
        if (this._keyboard != null) {
            this._keyboard.popHandler();
        }
        keyboard.pushHandler(handler);
        this._keyboard = keyboard;
    }

    this.container.classList.remove('hidden');
    this.update(text, fill_amount);
};

Banner.prototype.update = function(text, fill_amount) {
    this.text.textContent = text;
    this.bar.style.width = fill_amount * 100 + '%';
};
