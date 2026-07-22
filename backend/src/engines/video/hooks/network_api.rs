pub fn hook_script() -> &'static str {
    r#"
(function(){
    if (window.__np_fetch_hooked) return;
    window.__np_fetch_hooked = true;

    // Hook fetch
    var originalFetch = window.fetch;
    window.fetch = function(input, init) {
        var url = typeof input === 'string' ? input : (input.url || input.href || '');
        console.log(JSON.stringify({type:'np-hook',event:'fetch',url:url}));
        return originalFetch.apply(this, arguments);
    };

    // Hook XMLHttpRequest
    var OriginalXHR = window.XMLHttpRequest;
    window.XMLHttpRequest = function() {
        var xhr = new OriginalXHR();
        var originalOpen = xhr.open;
        xhr.open = function(method, url) {
            xhr._np_url = url;
            return originalOpen.apply(this, arguments);
        };
        var originalSend = xhr.send;
        xhr.send = function() {
            console.log(JSON.stringify({type:'np-hook',event:'xhr',url:xhr._np_url||''}));
            return originalSend.apply(this, arguments);
        };
        return xhr;
    };

    console.log(JSON.stringify({type:'np-hook',event:'hooked',target:'fetch+XHR'}));
})()
"#
}
