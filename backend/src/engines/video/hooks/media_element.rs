pub fn hook_script() -> &'static str {
    r#"
(function(){
    if (window.__np_media_hooked) return;
    window.__np_media_hooked = true;

    var originalPlay = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function() {
        console.log(JSON.stringify({type:'np-hook',event:'play',src:this.currentSrc}));
        return originalPlay.apply(this, arguments);
    };

    var originalPause = HTMLMediaElement.prototype.pause;
    HTMLMediaElement.prototype.pause = function() {
        console.log(JSON.stringify({type:'np-hook',event:'pause'}));
        return originalPause.apply(this, arguments);
    };

    var originalLoad = HTMLMediaElement.prototype.load;
    HTMLMediaElement.prototype.load = function() {
        console.log(JSON.stringify({type:'np-hook',event:'load'}));
        return originalLoad.apply(this, arguments);
    };

    // 监听已有 video 元素
    document.addEventListener('play', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'play_event',src:e.target.currentSrc}));
        }
    }, true);

    document.addEventListener('playing', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'playing_event',src:e.target.currentSrc}));
        }
    }, true);

    document.addEventListener('pause', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'pause_event'}));
        }
    }, true);

    document.addEventListener('ended', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'ended_event'}));
        }
    }, true);

    document.addEventListener('error', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'error_event',msg:e.target.error?.message||'unknown'}));
        }
    }, true);

    document.addEventListener('waiting', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'waiting_event'}));
        }
    }, true);

    document.addEventListener('canplay', function(e) {
        if (e.target.tagName === 'VIDEO') {
            console.log(JSON.stringify({type:'np-hook',event:'canplay_event'}));
        }
    }, true);

    console.log(JSON.stringify({type:'np-hook',event:'hooked',target:'HTMLMediaElement'}));
})()
"#
}
