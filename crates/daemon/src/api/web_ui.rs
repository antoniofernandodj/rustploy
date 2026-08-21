//! Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa
//! ao client iced (`rustploy-gui`), que estava sendo bloqueado pelo Windows
//! Defender. Fala com o daemon pelo MESMO protocolo HTTP/JSON + SSE que o
//! client iced já usa (`POST /api/rpc`, `GET /api/events`) — este módulo só
//! entrega o app shell (HTML/CSS/JS/ícones), pré-minificado e pré-gzipado em
//! tempo de build (ver `../../build.rs`), sem I/O nem CPU extra em runtime.
//!
//! Roteamento por caminho fixo, plugado em `http_api.rs::handle` ANTES do
//! gate de Bearer token — mesmo tratamento das rotas públicas de webhook/
//! OAuth, porque o usuário ainda não tem o token quando a página carrega (ele
//! loga *depois* de abrir). Isso é seguro: só entrega estático, sem dado
//! nenhum — todo acesso a dado de verdade continua exigindo o Bearer em
//! `/api/rpc` e `/api/events`, como já fazia o client iced.

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{Response, StatusCode};

type ApiBody = BoxBody<Bytes, std::convert::Infallible>;

/// Um arquivo do app shell, já processado (minificado + gzipado) em tempo de
/// build — ver `Asset` gerado por `build.rs` em `$OUT_DIR/webui_assets.rs`.
pub struct Asset {
    pub route: &'static str,
    pub content_type: &'static str,
    /// Hash de conteúdo (FNV-1a hex), usado como `ETag`.
    pub etag: &'static str,
    /// `true` para o HTML de entrada e o service worker: precisam revalidar
    /// sempre, para uma atualização do daemon propagar sem exigir
    /// hard-refresh. Os demais assets são imutáveis (o conteúdo só muda
    /// quando o build muda, e nesse caso são simplesmente outros bytes na
    /// mesma rota — o `ETag` cobre esse caso).
    pub no_cache: bool,
    /// Bytes já comprimidos com gzip.
    pub gz: &'static [u8],
}

static ASSETS: &[Asset] = include!(concat!(env!("OUT_DIR"), "/webui_assets.rs"));

/// Serve `path` se casar com algum asset embutido do app shell; `None` se a
/// rota não pertencer à web UI (o chamador cai no 404 padrão da API).
pub fn serve(path: &str) -> Option<Response<ApiBody>> {
    let asset = ASSETS.iter().find(|a| a.route == path)?;
    let cache_control = if asset.no_cache {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, asset.content_type)
            .header(hyper::header::CONTENT_ENCODING, "gzip")
            .header(hyper::header::CACHE_CONTROL, cache_control)
            .header(hyper::header::ETAG, format!("\"{}\"", asset.etag))
            .body(Full::new(Bytes::from_static(asset.gz)).boxed())
            .expect("resposta estática bem-formada"),
    )
}

/// Fase 0 do plano de convergência de templates (`docs/plano-convergencia-
/// templates-gui-webui.md`): equivalente web de
/// `crates/rustploy-gui/tests/templates_render.rs` — em vez de reimplementar
/// um servidor de estáticos, serve os MESMOS bytes que o daemon serve em
/// produção (`serve()` acima, já gzipados/minificados pelo `build.rs`) e
/// dirige um Chrome headless de verdade contra eles. Não passa por login/SSE
/// — como o teste iced, semeia o estado direto (aqui, `Alpine.store('app')`
/// em vez de `GlacierUI::define_data`) e varre as telas.
#[cfg(test)]
mod headless_tests {
    use super::serve;
    use bytes::Bytes;
    use chromiumoxide::browser::{Browser, BrowserConfig};
    use futures::StreamExt;
    use http_body_util::{combinators::BoxBody, BodyExt, Full};
    use hyper::{server::conn::http1, service::service_fn, Request, Response, StatusCode};
    use hyper_util::rt::TokioIo;
    use std::time::Duration;
    use tokio::net::TcpListener;

    type ApiBody = BoxBody<Bytes, std::convert::Infallible>;

    /// Sobe um servidor HTTP mínimo (mesmo padrão de accept-loop de
    /// `http_api.rs::listen`) que só chama `serve()` — os mesmos bytes que o
    /// daemon real serve, sem precisar de `AppState` (Docker/DB/etc.), que
    /// `serve()` não usa.
    async fn spawn_static_server() -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind do servidor estático de teste");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                tokio::spawn(async move {
                    let svc = service_fn(|req: Request<hyper::body::Incoming>| async move {
                        let resp: Response<ApiBody> = serve(req.uri().path()).unwrap_or_else(|| {
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Full::new(Bytes::from_static(b"not found")).boxed())
                                .expect("404 bem-formado")
                        });
                        Ok::<_, std::convert::Infallible>(resp)
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), svc)
                        .await;
                });
            }
        });
        addr
    }

    /// `cargo test` roda os 3 testes deste módulo em threads concorrentes;
    /// sem um `user_data_dir` próprio, todos os Chrome apontam pro mesmo
    /// perfil default do chromiumoxide e o segundo a chegar morre com
    /// "Failed to create SingletonLock: File exists" — cada `launch()`
    /// precisa do seu diretório.
    static USER_DATA_DIR_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    async fn launch() -> (Browser, tokio::task::JoinHandle<()>) {
        let n = USER_DATA_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let user_data_dir = std::env::temp_dir().join(format!(
            "rustploy-webui-test-{}-{n}",
            std::process::id()
        ));
        let config = BrowserConfig::builder()
            .chrome_executable("/usr/bin/google-chrome")
            .no_sandbox()
            .user_data_dir(user_data_dir)
            .build()
            .expect("BrowserConfig");
        let (browser, mut handler) = Browser::launch(config)
            .await
            .expect("google-chrome deve estar instalado (ver docs/inventario-paridade-telas.md)");
        // A stream de eventos CDP precisa ser drenada continuamente — sem
        // isso nenhum comando (navigate/evaluate) recebe resposta. Mesmo
        // padrão do exemplo oficial do chromiumoxide.
        let h = tokio::spawn(async move {
            while let Some(ev) = handler.next().await {
                let _ = ev;
            }
        });
        (browser, h)
    }

    /// Injeta um coletor de erros ANTES de qualquer script da página rodar
    /// (`Page.addScriptToEvaluateOnNewDocument`), pra pegar erros de módulo
    /// ES (`app.js`) que disparam antes do nosso primeiro `evaluate()`.
    const ERROR_COLLECTOR_JS: &str = r#"
        window.__errors = [];
        window.addEventListener('error', (e) => window.__errors.push(String(e.message || e.error)));
        window.addEventListener('unhandledrejection', (e) => window.__errors.push('rejection: ' + String(e.reason)));
    "#;

    /// Abre uma aba em branco, registra o coletor de erros e SÓ ENTÃO navega
    /// pra `index.html` — nessa ordem, pra `evaluate_on_new_document` valer
    /// já no primeiro load (`browser.new_page(url)` navegaria direto, o que
    /// deixaria a carga inicial sem o coletor).
    async fn open_page(browser: &Browser, addr: std::net::SocketAddr) -> chromiumoxide::Page {
        let page = browser.new_page("about:blank").await.expect("new_page");
        page.evaluate_on_new_document(ERROR_COLLECTOR_JS)
            .await
            .expect("injeta o coletor de erros");
        page.goto(format!("http://{addr}/")).await.expect("goto");
        let _ = page.wait_for_navigation().await;
        page
    }

    /// Espera até `Alpine.store('app')` existir — só depois do evento
    /// `load` (ver o comentário no fim de `webui/app.js`: os módulos
    /// registram os `Alpine.data`/`Alpine.store` primeiro, `Alpine.start()`
    /// só dispara depois).
    async fn wait_alpine_ready(page: &chromiumoxide::Page) {
        for _ in 0..100 {
            if let Ok(r) = page
                .evaluate("typeof window.Alpine !== 'undefined' && !!Alpine.store('app')")
                .await
            {
                if r.into_value::<bool>().unwrap_or(false) {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("Alpine.store('app') não ficou pronto a tempo");
    }

    /// Snapshot mínimo — mesmo formato que `http_api.rs::snapshot()` monta e
    /// `app.js::applySnapshot` consome (`kind` é ignorado por `applySnapshot`,
    /// só o `onStreamEvent` do SSE real olha pra ele). Cobre o suficiente
    /// pra cada tela ter pelo menos uma linha de tabela — mesmo espírito das
    /// fixtures de `templates_render.rs`, sem tentar ser exaustivo.
    const SNAPSHOT_JS: &str = r#"
        {
            status: { version: '0.0.0-test', uptime_secs: 3600, services_running: 1, services_total: 1 },
            deployments: [{ service_name: 'api', project_name: 'acme',
                deployment: { state: 'Live', started_at: new Date().toISOString() } }],
            projects: [{ id: 'prj_1', name: 'acme', description: '', env_vars: [], env_comments: [], secrets: ['GITHUB_TOKEN'] }],
            services: [{ project_name: 'acme', service: {
                id: 'svc_1', status: 'Running', live_container_id: 'abc123', containers: [],
                spec: { name: 'api', project_id: 'prj_1', port: 8080, replicas: 1, env_vars: [], domains: [],
                    source: { Git: { url: 'https://github.com/acme/api.git', branch: 'main', root_path: '.',
                        watch_paths: [], submodules: false, dockerfile_path: 'Dockerfile', build_context: '.',
                        build_stage: null, credentials: null, username: null, provider_id: null } },
                    healthcheck: { kind: 'None', interval_secs: 5, timeout_secs: 3, retries: 10, start_period_secs: 5 } } } }],
            docker_images: [], docker_volumes: [], docker_networks: [], docker_containers: [],
            jobs: [], engine: { paused: false, queued: [], active: [], recent: [] },
            registry_status: { enabled: true, domain: '', port: 5100 }, registry_repos: [],
        }
    "#;

    /// Semeia a sessão (bypassa login/SSE, mesma ideia de `boot()` +
    /// `define_data` em `templates_render.rs`) e roda `applySnapshot` — o
    /// mesmo método que o SSE real chama, então `dataLoading`/`jobsById`
    /// saem consistentes.
    async fn seed_connected(page: &chromiumoxide::Page) {
        wait_alpine_ready(page).await;
        let js = format!(
            r#"(() => {{
                const s = Alpine.store('app');
                s.connected = true;
                s.screen = 'shell';
                s.api = {{ baseUrl: location.origin, token: 't',
                    rpc: async () => ({{ ok: false, error: 'stub' }}),
                    rpcChecked: async () => ({{ ok: false, error: 'stub' }}) }};
                s.applySnapshot({SNAPSHOT_JS});
            }})()"#
        );
        page.evaluate(js).await.expect("seed do Alpine store");
    }

    /// `true` se o elemento existir e estiver de fato visível (`x-show`
    /// escreve `display:none` inline; `offsetParent` é `null` tanto pra
    /// `display:none` quanto pra ancestral escondido — cobre os dois).
    ///
    /// Faz polling (não uma checagem só): Alpine aplica `x-show`/`x-if` num
    /// microtask agendado pelo próprio scheduler dele, não sincronamente na
    /// mutação do store — então um `evaluate()` (round-trip CDP) imediatamente
    /// depois de mudar `view` pode chegar antes desse microtask rodar. Sem
    /// isso, o teste ficava flaky (falhava numa tela aleatória do sweep a
    /// cada corrida).
    async fn visible(page: &chromiumoxide::Page, selector: &str) -> bool {
        let js = format!(
            "(() => {{ const el = document.querySelector({selector:?}); return !!el && el.offsetParent !== null; }})()"
        );
        for _ in 0..20 {
            let ok = page
                .evaluate(js.as_str())
                .await
                .ok()
                .and_then(|r| r.into_value::<bool>().ok())
                .unwrap_or(false);
            if ok {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }

    async fn errors(page: &chromiumoxide::Page) -> Vec<String> {
        page.evaluate("window.__errors || []")
            .await
            .ok()
            .and_then(|r| r.into_value::<Vec<String>>().ok())
            .unwrap_or_default()
    }

    /// Varre as telas de nível superior (equivalente ao loop `for view in
    /// [...]` de `templates_render.rs::all_screens_and_service_tabs_render`)
    /// e confirma que cada uma monta (`x-data` fica visível) sem erro de JS.
    /// `support` não existe na webui (só no client iced — ver
    /// `docs/inventario-paridade-telas.md`), então não entra aqui.
    #[tokio::test]
    async fn todas_as_telas_principais_renderizam() {
        let addr = spawn_static_server().await;
        let (browser, _handler) = launch().await;
        let page = open_page(&browser, addr).await;

        // Login (tela default, antes de qualquer seed).
        wait_alpine_ready(&page).await;
        assert!(
            visible(&page, ".login_wrap").await,
            "tela de login deveria estar visível antes do login"
        );

        seed_connected(&page).await;

        let views: &[(&str, &str)] = &[
            ("deployments", "[x-data=\"dashboard\"]"),
            ("projects", "[x-data=\"projects\"]"),
            ("monitoring", "[x-data=\"monitoring\"]"),
            ("ingress", "[x-data=\"ingress\"]"),
            ("docker", "[x-data=\"docker\"]"),
            ("schedules", "[x-data=\"schedules\"]"),
            ("settings", "[x-data=\"settings\"]"),
            ("deploy_engine", "[x-data=\"deployEngine\"]"),
        ];
        for (view, selector) in views {
            page.evaluate(format!("Alpine.store('app').view = {view:?}"))
                .await
                .unwrap_or_else(|e| panic!("set view={view}: {e}"));
            assert!(
                visible(&page, selector).await,
                "view={view} deveria estar visível (seletor {selector})"
            );
        }
        assert_eq!(errors(&page).await, Vec::<String>::new(), "sem erros de JS no console");
    }

    /// Aba "Projeto aberto" — `view` chama-se `project` na webui contra
    /// `project_services` no client iced (divergência mecânica de nome, ver
    /// `docs/inventario-paridade-telas.md`); precisa de `selectedProjectId`.
    #[tokio::test]
    async fn project_detail_renderiza() {
        let addr = spawn_static_server().await;
        let (browser, _handler) = launch().await;
        let page = open_page(&browser, addr).await;
        seed_connected(&page).await;

        page.evaluate("Alpine.store('app').view = 'project'; Alpine.store('app').selectedProjectId = 'prj_1'")
            .await
            .expect("abre o projeto");
        assert!(visible(&page, "[x-data=\"projectDetail\"]").await, "project_detail deveria estar visível");
        assert_eq!(errors(&page).await, Vec::<String>::new(), "sem erros de JS no console");
    }

    /// Detalhe de serviço + a aba General (edição de origem — Compose / Git
    /// / conta conectada / Zip), implementada nesta sessão. `serviceDetail`
    /// não é derivado de `snap` (o real vem de `fetchServiceDetail`), então
    /// precisa ser semeado à mão — mesma razão de `m.define_data("svc_env",
    /// ...)` no teste iced.
    #[tokio::test]
    async fn service_detail_e_general_tab_renderizam() {
        let addr = spawn_static_server().await;
        let (browser, _handler) = launch().await;
        let page = open_page(&browser, addr).await;
        seed_connected(&page).await;

        let open_service = r#"(() => {
            const s = Alpine.store('app');
            s.view = 'service';
            s.selectedServiceId = 'svc_1';
            s.serviceTab = 'general';
            s.serviceLoading = false;
            s.serviceDeployments = [];
            s.serviceDetail = {
                id: 'svc_1', status: 'Running', live_container_id: 'abc123',
                spec: { name: 'api', port: 8080, replicas: 1, project_id: 'prj_1', env_vars: [], domains: [],
                    source: { Git: { url: 'https://github.com/acme/api.git', branch: 'main', root_path: '.',
                        watch_paths: ['src'], submodules: false, dockerfile_path: 'Dockerfile', build_context: '.',
                        build_stage: null, credentials: null, username: null, provider_id: null } },
                    healthcheck: { kind: 'None', interval_secs: 5, timeout_secs: 3, retries: 10, start_period_secs: 5 } },
            };
        })()"#;
        page.evaluate(open_service).await.expect("abre o serviço");
        assert!(visible(&page, "[x-data=\"serviceDetail\"]").await, "service_detail deveria estar visível");
        assert!(
            visible(&page, "[x-data=\"serviceDetail\"] input[x-model=\"fRepoUrl\"]").await,
            "aba General deveria mostrar o form Git (sourceKind=Git)"
        );

        // As 3 sub-abas Provider (mesma cobertura do sweep de prov_tab em
        // templates_render.rs) — cada uma com um marcador só seu no DOM.
        let prov_tabs: &[(&str, &str)] = &[
            ("git", "[x-data=\"serviceDetail\"] input[x-model=\"fRepoUrl\"]"),
            ("gitea", "[x-data=\"serviceDetail\"] select[x-model=\"giteaProviderId\"]"),
            ("zip", "[x-data=\"serviceDetail\"] input[type=\"file\"]"),
        ];
        for (tab, selector) in prov_tabs {
            page.evaluate(format!(
                "Alpine.$data(document.querySelector('[x-data=\"serviceDetail\"]')).setProvTab({tab:?})"
            ))
            .await
            .unwrap_or_else(|e| panic!("setProvTab({tab}): {e}"));
            assert!(visible(&page, selector).await, "prov_tab={tab} deveria mostrar {selector}");
        }

        // Compose: troca a origem inteira e confirma que o textarea aparece.
        page.evaluate(
            r#"(() => {
                const s = Alpine.store('app');
                s.serviceDetail = JSON.parse(JSON.stringify(s.serviceDetail));
                s.serviceDetail.spec.source = { Compose: { content: 'services:\n  web:\n    image: nginx\n' } };
                Alpine.$data(document.querySelector('[x-data="serviceDetail"]')).initGeneralForm();
            })()"#,
        )
        .await
        .expect("troca pra origem Compose");
        // Seletor ESCOPADO ao componente: `composeText` também é o nome do
        // campo local do wizard "Novo serviço" (`newService`, passo Compose
        // colado) — `document.querySelector` sem escopo pegava aquele
        // textarea (fora de tela, `x-show=false`) em vez do nosso.
        assert!(
            visible(&page, "[x-data=\"serviceDetail\"] textarea[x-model=\"composeText\"]").await,
            "origem Compose deveria mostrar o editor de texto"
        );

        assert_eq!(errors(&page).await, Vec::<String>::new(), "sem erros de JS no console");
    }
}
