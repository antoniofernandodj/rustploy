# Notificação nativa do SO ao concluir um deploy

Quando um deploy chega a um estado terminal (`Live`/`Failed`), o rustploy-gui
emite uma **notificação nativa do sistema operacional** (central de notificações
do GNOME/freedesktop no Linux, WinRT no Windows, `NSUserNotification` no macOS).
Assim o usuário dispara o deploy, vai fazer outra coisa, e é avisado sem precisar
estar com a janela à frente.

## Onde mora cada peça

- **Disparo**: `views/scripts/handlers/stream.luau`, no ramo `DeployStateChanged`
  do consumidor SSE — no bloco terminal, **fora** do gate de `selected_service`
  (o ponto da notificação do SO é justamente avisar mesmo com outra tela aberta).
- **Nome do serviço**: capturado ao armar o deploy em `State.deploy_track.name`
  (`handlers/services.luau::start_deploy`), porque o evento `DeployStateChanged`
  só carrega o `service_id`.
- **API**: função global `notify({ title, body, app_name?, icon? })` do
  **glacier-ui 0.44+** (ver o `CHANGELOG` do glacier). No **Linux**, o glacier
  emite via **`notify-send` (subprocesso)**, com fallback para `notify-rust`
  in-process se o `notify-send` não existir; em Windows/macOS é `notify-rust`
  in-process (WinRT / `NSUserNotification`). Sempre num `std::thread`, fora da
  thread de UI.

## Por que subprocesso (`notify-send`) e não in-process — NÃO reverter

O caminho óbvio (emitir a notificação *in-process* via `notify-rust`) **não
aparece** em alguns ambientes — reproduzido num **GNOME 46 (Ubuntu 24.04)**, sem
nenhuma extensão de filtro de notificações. O compositor associa a notificação
fdo ao app pelo **PID → janela → `app_id`** (aqui `rustploy-gui`, casando com
`/usr/share/applications/rustploy-gui.desktop`) e a **descarta silenciosamente**,
mesmo com o app habilitado nas configurações (`enable=true`): o `.show()` retorna
`Ok`, nenhum erro é logado, e nada aparece.

Investigação que levou até aqui (todas as variações testadas com `notify-rust`
num binário avulso):

- `notify-send` "plain" → **aparece**; `notify-send --hint=desktop-entry:<app>`
  (qualquer app, inclusive Chrome) → **descartada**.
- `appname="rustploy-gui"` (default do notify-rust = nome do exe) → **descartada**;
  `appname="Rustploy"` (nome de exibição) → **aparece** — no binário avulso.
- **Mas** no app real, mesmo com `appname="Rustploy"`, a notificação in-process
  **sumia** — porque a associação vem do `app_id` da **janela**, não do `appname`.
  Nem urgência crítica furava.
- Um **subprocesso sem janela** (`notify-send`, processo filho) não é associado a
  nenhum app pelo compositor → **é exibido**. Essa é a solução.

O `app_name="Rustploy"` e o `icon="rustploy-gui"` nas chamadas viram flags do
`notify-send` (`--app-name`/`--icon`) — o ícone existe no tema hicolor. Este
comportamento do GNOME é anômalo (o esperado seria exibir com o ícone do app),
mas o subprocesso é a forma robusta e sob nosso controle de garantir a entrega.
