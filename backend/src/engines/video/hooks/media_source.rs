pub fn hook_script() -> &'static str {
    r#"
(function(){
    if (window.__np_mse_hooked) return;
    window.__np_mse_hooked = true;

    if (typeof MediaSource === 'undefined' && typeof WebKitMediaSource !== 'undefined') {
        window.MediaSource = window.WebKitMediaSource;
    }

    if (typeof MediaSource !== 'undefined') {
        var OriginalMediaSource = MediaSource;

        var originalAddSourceBuffer = OriginalMediaSource.prototype.addSourceBuffer;
        if (originalAddSourceBuffer) {
            OriginalMediaSource.prototype.addSourceBuffer = function(type) {
                console.log(JSON.stringify({type:'np-hook',event:'mse_addSourceBuffer',mime:type}));
                return originalAddSourceBuffer.apply(this, arguments);
            };
        }
    }

    if (typeof SourceBuffer !== 'undefined') {
        var originalAppendBuffer = SourceBuffer.prototype.appendBuffer;
        if (originalAppendBuffer) {
            SourceBuffer.prototype.appendBuffer = function(data) {
                var size = data ? (data.byteLength || data.length || 0) : 0;
                console.log(JSON.stringify({type:'np-hook',event:'mse_appendBuffer',size:size}));
                return originalAppendBuffer.apply(this, arguments);
            };
        }
    }

    console.log(JSON.stringify({type:'np-hook',event:'hooked',target:'MediaSource'}));
})()
"#
}
