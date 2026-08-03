// sw.js — service worker do PWA Rustploy.
//
// Estratégia: cache-first para o "app shell" (HTML/CSS/JS/ícones — tudo
// versionado e imutável, ver crates/daemon/build.rs), rede sempre para
// `/api/*` (dado dinâmico, nunca deve ficar preso em cache). O nome do cache
// carrega o hash do build (`__CACHE_VERSION__`, substituído em build.rs) — a
// cada build com conteúdo novo, o cache antigo é descartado no `activate`.

const CACHE_NAME = "rustploy-webui-__CACHE_VERSION__";

const APP_SHELL = [
  "/",
  "/app.css",
  "/app.js",
  "/fmt.js",
  "/net/api.js",
  "/net/sse.js",
  "/screens/login.js",
  "/screens/dashboard.js",
  "/screens/projects.js",
  "/screens/project_detail.js",
  "/screens/new_service.js",
  "/screens/service_detail.js",
  "/manifest.webmanifest",
  "/icons/icon-192.png",
  "/icons/icon-512.png",
  "/icons/icon-512-maskable.png",
  "/icons/apple-touch-icon.png",
  "/icons/favicon.png",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(APP_SHELL)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  // Dado dinâmico: nunca intercepta /api/* — sempre vai pra rede.
  if (url.pathname.startsWith("/api/")) return;
  if (event.request.method !== "GET") return;

  event.respondWith(
    caches.match(event.request).then((cached) => {
      if (cached) return cached;
      return fetch(event.request).then((res) => {
        // Só cacheia respostas do mesmo domínio, com sucesso — deixa o Alpine
        // CDN (cross-origin) fora do controle deste SW.
        if (res.ok && url.origin === self.location.origin) {
          const clone = res.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
        }
        return res;
      });
    })
  );
});
