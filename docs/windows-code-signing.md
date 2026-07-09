# Assinatura de código no Windows (rustploy-gui)

Por que o `.exe` é bloqueado ao baixar/rodar no Windows, e o que dá para fazer
— com foco no que é **gratuito**.

## O que está acontecendo

O aviso "O controle inteligente de aplicativos bloqueou um aplicativo que pode
não ser seguro" vem do **Smart App Control (SAC)** do Windows 11 — o modo mais
agressivo. Além dele existe o **SmartScreen** clássico ("Editor: Desconhecido").
Os dois decidem por **reputação**, não só por "tem assinatura?":

- **SmartScreen (comum)**: pesa metadados do binário, se está assinado, e a
  reputação acumulada do arquivo/certificado. Um cert OV novo ainda passa por um
  período de "aquecimento" de reputação.
- **Smart App Control**: praticamente só libera binários assinados por um
  certificado com **reputação estabelecida** (na prática, **EV**) ou já vetados
  pela nuvem da Microsoft. **Nenhuma opção gratuita passa direto pelo SAC ligado.**

## O teto de cada opção

| Opção | Custo | Resolve SmartScreen comum? | Resolve Smart App Control? |
|---|---|---|---|
| Metadados + manifest no `.exe` | grátis | melhora (não garante) | não |
| Auto-assinatura (self-signed) | grátis | só em máquinas que importam o cert | não |
| **SignPath (tier OSS)** | grátis (repo público) | sim, com reputação ao longo do tempo | não de imediato |
| GitHub Releases + winget | grátis | ajuda (reputação/canal confiável) | não |
| Cert **EV** | pago (~US$/ano + token) | sim | **sim** |

> Resumo honesto: **de graça** dá pra deixar o download muito menos assustador
> na maioria das máquinas (SmartScreen comum, "editor desconhecido"), mas o
> Smart App Control **ligado** só cede com EV ou desligando o SAC (irreversível
> — não peça isso a usuários finais; use só em VM de teste).

## O que já está implementado no repo

### 1. Metadados + manifest embutidos no `.exe` (grátis, sempre ativo)

- `crates/rustploy-gui/assets/rustploy.rc` — bloco `VERSIONINFO` (CompanyName,
  ProductName, versão derivada de `CARGO_PKG_VERSION` pelo `build.rs`) + o ícone.
  **Só ASCII nos `VALUE`**: o `llvm-rc` do cross-build não interpreta UTF-8 em
  `VERSIONINFO` (um travessão `—` quebra o build).
- `crates/rustploy-gui/assets/application.manifest` — identidade, `asInvoker`
  (sem UAC), DPI PerMonitorV2, long paths. Embutido como `RT_MANIFEST`.

Nada a fazer: sai pronto em `make rustploy-gui-windows` / no CI.

### 2. Auto-assinatura para testes internos (grátis, opcional)

Roda no próprio Linux via `osslsigncode` (não precisa de Windows):

```bash
sudo apt install osslsigncode
make rustploy-gui-windows          # compila o .exe
make rustploy-gui-windows-sign     # gera cert self-signed + assina + timestamp
```

Gera `dist/rustploy-selfsign.cer`. Para o app ser aceito **numa máquina Windows
específica**, importe esse `.cer` em *Autoridades de Certificação Raiz
Confiáveis* **e** em *Editores Confiáveis* (Certificados do Computador Local).
Serve para time/uso interno — **não** para o público e **não** passa no SAC.

### 3. SignPath (tier OSS) no CI — o caminho grátis "de verdade"

`.github/workflows/release.yml` já tem o job `sign-gui-windows`, que fica
**inerte** até você registrar o projeto. Ele só roda quando a variável de
repositório `SIGNPATH_ORGANIZATION_ID` existe; sem isso, o pipeline empacota o
`.exe` cru normalmente.

Passos de registro (uma vez):

1. Cadastre o projeto open source em **https://signpath.org** (SignPath
   Foundation, gratuito para OSS). Aguarde a aprovação.
2. No projeto SignPath, crie:
   - um **Project** com slug `rustploy` (ou ajuste o `project-slug` no workflow);
   - uma **Artifact configuration** para um `.exe` avulso (slug `exe`);
   - uma **Signing policy** (slug `release-signing`).
3. No GitHub, em *Settings → Secrets and variables → Actions*:
   - **Variables**: `SIGNPATH_ORGANIZATION_ID` (obrigatória — é o gatilho do job),
     e opcionalmente `SIGNPATH_SIGNING_POLICY_SLUG` / `SIGNPATH_ARTIFACT_CONFIG_SLUG`
     se você usou slugs diferentes.
   - **Secrets**: `SIGNPATH_API_TOKEN`.
4. Publique uma tag `vX.Y.Z`. O fluxo: `build-gui-windows` (compila e sobe o
   `.exe` cru) → `sign-gui-windows` (SignPath assina) → `package-gui-windows`
   (monta a árvore de assets ao redor do `.exe` assinado e gera o `.zip`).

> O SignPath assina na **infra deles** a partir do artefato do run do GitHub
> Actions; a chave privada nunca toca no CI. Por isso o build precisa rodar no
> GitHub Actions (não adianta assinar um `.exe` que você compilou localmente e
> subiu à mão — a policy amarra ao run de origem).

## Recomendação de sequência (tudo grátis)

1. Já feito: metadados/manifest no binário.
2. Registrar o projeto no SignPath (itens acima) e assinar os releases.
3. Publicar via **GitHub Releases** e, com o tempo, submeter ao **winget** —
   reputação de canal + histórico é o que amansa o SmartScreen comum.
4. Enquanto a reputação não acumula, orientar quem baixa: `Unblock-File` no
   PowerShell, ou "Mais informações → Executar assim mesmo" no SmartScreen.

Só suba para um cert **EV pago** se precisar mesmo passar pelo **Smart App
Control ligado** sem fricção nenhuma.
