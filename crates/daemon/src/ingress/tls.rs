use anyhow::{Result, anyhow};
use instant_acme::{
    Account, AccountCredentials, ChallengeType, Identifier, NewAccount, NewOrder, OrderStatus,
};
use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use rustls::{
    ServerConfig,
    crypto::ring::{default_provider, sign::any_supported_type},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use shared::config::AcmeConfig;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// token → key_authorization: compartilhado com o handler HTTP para servir challenges.
pub type ChallengeStore = Arc<Mutex<HashMap<String, String>>>;

struct SniResolver {
    certs: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for SniResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.certs.read().map(|c| c.len()).unwrap_or(0);
        write!(f, "SniResolver({n} certs)")
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let name = client_hello.server_name()?;
        self.certs.read().ok()?.get(name).cloned()
    }
}

pub struct TlsManager {
    cert_dir: PathBuf,
    /// Tokens ACME pendentes: token → key_authorization
    pub challenges: ChallengeStore,
    resolver: Arc<SniResolver>,
    /// Arc estável — SniResolver é atualizado internamente sem trocar o ServerConfig.
    server_config: Arc<ServerConfig>,
    acme_config: Mutex<AcmeConfig>,
}

impl TlsManager {
    pub fn new(cert_dir: PathBuf, acme_config: AcmeConfig) -> Result<Self> {
        std::fs::create_dir_all(&cert_dir)?;

        let resolver = Arc::new(SniResolver {
            certs: RwLock::new(HashMap::new()),
        });

        let server_config = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| anyhow!("TLS protocol config: {e}"))?
                .with_no_client_auth()
                .with_cert_resolver(resolver.clone()),
        );

        let mgr = Self {
            cert_dir,
            challenges: Arc::new(Mutex::new(HashMap::new())),
            resolver,
            server_config,
            acme_config: Mutex::new(acme_config),
        };

        mgr.load_all_from_disk();
        Ok(mgr)
    }

    /// Retorna um TlsAcceptor que reutiliza o ServerConfig imutável.
    /// Novos certificados são injetados no SniResolver sem recriar o acceptor.
    pub fn tls_acceptor(&self) -> tokio_rustls::TlsAcceptor {
        tokio_rustls::TlsAcceptor::from(self.server_config.clone())
    }

    /// Ativa ACME dinamicamente — sem precisar reiniciar o daemon.
    pub fn enable_acme(&self, email: String) {
        let mut cfg = self.acme_config.lock().unwrap();
        cfg.enabled = true;
        cfg.email = Some(email);
        info!("TLS: ACME ativado dinamicamente");
    }

    /// Desativa ACME (chamado quando o e-mail é removido).
    pub fn disable_acme(&self) {
        let mut cfg = self.acme_config.lock().unwrap();
        cfg.enabled = false;
        cfg.email = None;
        info!("TLS: ACME desabilitado");
    }

    /// Garante que exista um certificado válido para `domain`.
    /// No-op se ACME estiver desabilitado ou se o cert existir e não expirar em 30 dias.
    pub async fn ensure_cert(&self, domain: &str) -> Result<()> {
        let (enabled, email, directory) = {
            let cfg = self.acme_config.lock().unwrap();
            (cfg.enabled, cfg.email.clone(), cfg.directory.clone())
        };

        info!(domain, acme_enabled = enabled, acme_email = ?email, acme_directory = %directory, "TLS: ensure_cert chamado");

        if !enabled {
            warn!(domain, "TLS: ACME desabilitado, ignorando provisionamento");
            return Ok(());
        }

        if self.cert_is_valid(domain) {
            info!(domain, "TLS: certificado já válido, sem ação necessária");
            return Ok(());
        }

        info!(domain, "TLS: nenhum certificado válido encontrado, iniciando provisionamento via ACME");

        let email = email.as_deref().unwrap_or("admin@localhost");
        info!(domain, email, directory = %directory, "TLS: carregando/criando conta ACME");

        let account = self.load_or_create_account(email, &directory).await?;
        info!(domain, "TLS: conta ACME pronta, criando nova order");

        let mut order = account
            .new_order(&NewOrder {
                identifiers: &[Identifier::Dns(domain.to_string())],
            })
            .await
            .map_err(|e| anyhow!("ACME new order: {e}"))?;

        info!(domain, "TLS: order criada, obtendo authorizations");

        let authorizations = order
            .authorizations()
            .await
            .map_err(|e| anyhow!("ACME authorizations: {e}"))?;

        info!(domain, count = authorizations.len(), "TLS: authorizations recebidas");

        let mut pending: Vec<(String, String)> = vec![];

        for auth in &authorizations {
            let challenge = auth
                .challenges
                .iter()
                .find(|c| c.r#type == ChallengeType::Http01)
                .ok_or_else(|| anyhow!("nenhum challenge HTTP-01 disponível para {domain}"))?;

            info!(domain, token = %challenge.token, challenge_url = %challenge.url, "TLS: challenge HTTP-01 encontrado, armazenando token");

            let key_auth = order.key_authorization(challenge);
            self.challenges
                .lock()
                .unwrap()
                .insert(challenge.token.clone(), key_auth.as_str().to_string());

            pending.push((challenge.url.clone(), challenge.token.clone()));
        }

        info!(domain, "TLS: notificando LE que challenges estão prontos");

        // Notifica LE que os challenges estão prontos
        for (url, token) in &pending {
            info!(domain, token = %token, url = %url, "TLS: set_challenge_ready");
            order
                .set_challenge_ready(url)
                .await
                .map_err(|e| anyhow!("ACME set_challenge_ready: {e}"))?;
        }

        info!(domain, "TLS: aguardando LE validar o challenge (poll a cada 3s, máx 90s)");

        // Aguarda o pedido ficar Ready (LE valida o challenge)
        let mut ready = false;
        for attempt in 0..30 {
            sleep(Duration::from_secs(3)).await;
            let state = order
                .refresh()
                .await
                .map_err(|e| anyhow!("ACME refresh: {e}"))?;
            debug!(domain, attempt, status = ?state.status, "TLS: poll order status");
            match state.status {
                OrderStatus::Ready => {
                    info!(domain, attempt, "TLS: order Ready — LE validou o challenge com sucesso");
                    ready = true;
                    break;
                }
                OrderStatus::Invalid => {
                    warn!(domain, attempt, "TLS: order Invalid — LE rejeitou o challenge");
                    self.remove_challenges(&pending);
                    return Err(anyhow!("ACME: order inválida para {domain} (LE rejeitou o challenge HTTP-01)"));
                }
                other => {
                    info!(domain, attempt, status = ?other, "TLS: aguardando...");
                }
            }
        }

        self.remove_challenges(&pending);

        if !ready {
            return Err(anyhow!(
                "ACME: timeout aguardando order Ready para {domain} (90s esgotados)"
            ));
        }

        info!(domain, "TLS: gerando chave privada e CSR");

        // Gera chave privada e CSR.
        // DistinguishedName vazio evita que rcgen coloque o CN padrão
        // "rcgen self signed cert" no CSR — o LE rejeita qualquer CN que
        // não seja um hostname válido (urn:acme:error:rejectedIdentifier).
        // Autoridades modernas usam apenas os SANs para validação.
        let key_pair = KeyPair::generate().map_err(|e| anyhow!("rcgen keygen: {e}"))?;
        let mut params = CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| anyhow!("rcgen params: {e}"))?;
        params.distinguished_name = DistinguishedName::new();
        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| anyhow!("rcgen CSR: {e}"))?;

        info!(domain, "TLS: finalizando order com CSR");

        // Finaliza o pedido
        order
            .finalize(csr.der())
            .await
            .map_err(|e| anyhow!("ACME finalize: {e}"))?;

        info!(domain, "TLS: aguardando certificado ficar disponível");

        // Aguarda o certificado ficar disponível
        let cert_chain_pem = loop {
            sleep(Duration::from_secs(3)).await;
            let order_status = {
                let state = order
                    .refresh()
                    .await
                    .map_err(|e| anyhow!("ACME refresh pós-finalize: {e}"))?;
                state.status
            };
            if matches!(order_status, OrderStatus::Invalid) {
                return Err(anyhow!("ACME: order inválida durante finalização para {domain}"));
            }
            if let Some(chain) = order
                .certificate()
                .await
                .map_err(|e| anyhow!("ACME certificate: {e}"))?
            {
                info!(domain, "TLS: certificado recebido do LE");
                break chain;
            }
            debug!(domain, status = ?order_status, "TLS: certificado ainda não disponível, aguardando...");
        };

        info!(domain, "TLS: salvando certificado em disco");
        let key_pem = key_pair.serialize_pem();
        self.save_cert(domain, &cert_chain_pem, &key_pem)?;

        info!(domain, "TLS: carregando certificado no SniResolver");
        let ck = self.parse_certified_key(cert_chain_pem.as_bytes(), key_pem.as_bytes())?;
        self.resolver
            .certs
            .write()
            .unwrap()
            .insert(domain.to_string(), ck);

        info!(domain, "TLS: certificado provisionado com sucesso — HTTPS ativo");
        Ok(())
    }

    /// Renova certificados que expiram em menos de 30 dias.
    pub async fn renew_expiring(&self) -> Result<Vec<String>> {
        let domains: Vec<String> = self
            .resolver
            .certs
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect();

        let mut renewed = vec![];
        for domain in domains {
            let cert_path = self.cert_dir.join(&domain).join("cert.pem");
            if self.cert_file_expires_soon(&cert_path) {
                info!(domain = %domain, "TLS: renovando certificado expirante");
                match self.ensure_cert(&domain).await {
                    Ok(_) => renewed.push(domain),
                    Err(e) => warn!(domain = %domain, error = %e, "TLS: falha na renovação"),
                }
            }
        }
        Ok(renewed)
    }

    // ─── Helpers privados ─────────────────────────────────────────────────────

    fn cert_is_valid(&self, domain: &str) -> bool {
        let in_memory = self.resolver.certs.read().unwrap().contains_key(domain);
        if !in_memory {
            return false;
        }
        let cert_path = self.cert_dir.join(domain).join("cert.pem");
        !self.cert_file_expires_soon(&cert_path)
    }

    fn cert_file_expires_soon(&self, path: &Path) -> bool {
        let Ok(meta) = std::fs::metadata(path) else {
            return true;
        };
        let Ok(modified) = meta.modified() else {
            return true;
        };
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::from_secs(u64::MAX));
        // Let's Encrypt emite por 90 dias; renova aos 60 dias de idade
        age > Duration::from_secs(60 * 24 * 3600)
    }

    async fn load_or_create_account(&self, email: &str, directory: &String) -> Result<Account> {
        let creds_path = self.cert_dir.join("acme-account.json");

        if let Ok(raw) = std::fs::read_to_string(&creds_path) {
            info!("TLS: credenciais ACME encontradas em {}, tentando restaurar conta", creds_path.display());
            match serde_json::from_str::<AccountCredentials>(&raw) {
                Ok(creds) => match Account::from_credentials(creds).await {
                    Ok(account) => {
                        info!("TLS: conta ACME restaurada com sucesso");
                        return Ok(account);
                    }
                    Err(e) => {
                        warn!(error = %e, "TLS: falha ao restaurar conta ACME das credenciais salvas, criando nova");
                    }
                },
                Err(e) => {
                    warn!(error = %e, "TLS: credenciais salvas inválidas (JSON corrompido?), criando nova conta");
                }
            }
        } else {
            info!("TLS: nenhuma credencial ACME salva, criando nova conta");
        }

        info!(email, directory = %directory, "TLS: criando nova conta ACME");
        let (account, credentials) = Account::create(
            &NewAccount {
                contact: &[&format!("mailto:{email}")],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            directory,
            None,
        )
        .await
        .map_err(|e| anyhow!("ACME create account: {e}"))?;

        std::fs::write(&creds_path, serde_json::to_string(&credentials)?)?;
        info!("TLS: conta ACME criada e salva em {}", creds_path.display());

        Ok(account)
    }

    fn save_cert(&self, domain: &str, cert_pem: &str, key_pem: &str) -> Result<()> {
        let dir = self.cert_dir.join(domain);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("cert.pem"), cert_pem)?;
        std::fs::write(dir.join("key.pem"), key_pem)?;
        Ok(())
    }

    fn parse_certified_key(
        &self,
        cert_pem: &[u8],
        key_pem: &[u8],
    ) -> Result<Arc<CertifiedKey>> {
        use rustls::pki_types::CertificateDer;

        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut &*cert_pem)
                .collect::<Result<_, _>>()
                .map_err(|e| anyhow!("parse cert PEM: {e}"))?;

        if certs.is_empty() {
            return Err(anyhow!("nenhum certificado encontrado no PEM"));
        }

        let key = rustls_pemfile::private_key(&mut &*key_pem)
            .map_err(|e| anyhow!("parse key PEM: {e}"))?
            .ok_or_else(|| anyhow!("nenhuma chave privada encontrada"))?;

        let signing_key =
            any_supported_type(&key).map_err(|e| anyhow!("tipo de chave não suportado: {e}"))?;

        Ok(Arc::new(CertifiedKey::new(certs, signing_key)))
    }

    fn load_all_from_disk(&self) {
        let Ok(entries) = std::fs::read_dir(&self.cert_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(domain) = path.file_name().and_then(|n| n.to_str()).map(str::to_string)
            else {
                continue;
            };
            if domain == "acme-account.json" {
                continue;
            }
            let cert_path = path.join("cert.pem");
            let key_path = path.join("key.pem");
            let Ok(cert_pem) = std::fs::read(&cert_path) else {
                continue;
            };
            let Ok(key_pem) = std::fs::read(&key_path) else {
                continue;
            };
            match self.parse_certified_key(&cert_pem, &key_pem) {
                Ok(ck) => {
                    self.resolver
                        .certs
                        .write()
                        .unwrap()
                        .insert(domain.clone(), ck);
                    info!(domain = %domain, "TLS: certificado carregado do disco");
                }
                Err(e) => {
                    warn!(domain = %domain, error = %e, "TLS: falha ao carregar cert do disco");
                }
            }
        }
    }

    fn remove_challenges(&self, pending: &[(String, String)]) {
        let mut store = self.challenges.lock().unwrap();
        for (_, token) in pending {
            store.remove(token);
        }
    }
}
