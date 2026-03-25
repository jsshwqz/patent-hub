const CACHE_NAME = 'patent-hub-v2';

const STATIC_ASSETS = [
    '/static/style.css',
    '/static/i18n.js',
    '/static/manifest.json'
];

self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
    );
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((names) =>
            Promise.all(
                names.filter((n) => n !== CACHE_NAME).map((n) => caches.delete(n))
            )
        )
    );
    self.clients.claim();
});

self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // Network-first for API calls
    if (url.pathname.startsWith('/api/')) {
        event.respondWith(
            fetch(event.request).catch(() => caches.match(event.request))
        );
        return;
    }

    // Cache-first for static assets
    if (url.pathname.startsWith('/static/')) {
        event.respondWith(
            caches.match(event.request).then((cached) =>
                cached || fetch(event.request).then((resp) => {
                    const clone = resp.clone();
                    caches.open(CACHE_NAME).then((c) => c.put(event.request, clone));
                    return resp;
                })
            )
        );
        return;
    }

    // Network-first for pages
    event.respondWith(
        fetch(event.request).catch(() => caches.match(event.request))
    );
});
