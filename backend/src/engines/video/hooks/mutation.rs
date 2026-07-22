pub fn hook_script() -> &'static str {
    r#"
(function(){
    if (window.__np_mutation_hooked) return;
    window.__np_mutation_hooked = true;

    var observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            mutation.addedNodes.forEach(function(node) {
                if (node.tagName === 'VIDEO') {
                    console.log(JSON.stringify({type:'np-hook',event:'mutation_video_added',src:node.currentSrc||node.src||''}));
                }
                // 递归检查子节点
                if (node.querySelectorAll) {
                    var videos = node.querySelectorAll('video');
                    videos.forEach(function(v) {
                        console.log(JSON.stringify({type:'np-hook',event:'mutation_video_added',src:v.currentSrc||v.src||''}));
                    });
                }
            });
        });
    });

    observer.observe(document.documentElement, {
        childList: true,
        subtree: true
    });

    console.log(JSON.stringify({type:'np-hook',event:'hooked',target:'MutationObserver'}));
})()
"#
}
