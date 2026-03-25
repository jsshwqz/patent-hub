// Patent Hub API Configuration
// On desktop: API server runs locally at localhost:3000
// On mobile APP: Connect to PC server via LAN IP
(function() {
    var saved = localStorage.getItem('patent_hub_server');
    if (saved) {
        window.API_BASE = saved;
    } else {
        window.API_BASE = '';
    }

    // Show connection prompt if no server configured and not on localhost
    if (!window.API_BASE && typeof window.__TAURI__ !== 'undefined') {
        // Running inside Tauri APP - need server address
        setTimeout(function() {
            var addr = prompt(
                'Patent Hub 移动版\n\n' +
                '请输入服务器地址（PC上运行的Patent Hub）：\n' +
                '例如：http://192.168.1.100:3000\n\n' +
                '提示：在PC上启动Patent Hub后，\n' +
                '查看控制台显示的 Mobile access 地址',
                'http://192.168.1.100:3000'
            );
            if (addr) {
                addr = addr.replace(/\/+$/, '');
                localStorage.setItem('patent_hub_server', addr);
                window.API_BASE = addr;
                location.reload();
            }
        }, 500);
    }

    // Override fetch to prepend API_BASE
    var originalFetch = window.fetch;
    window.fetch = function(url, options) {
        if (typeof url === 'string' && url.startsWith('/api/')) {
            url = window.API_BASE + url;
        }
        return originalFetch.call(this, url, options);
    };
})();
