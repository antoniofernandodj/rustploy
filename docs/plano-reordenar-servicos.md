# Reordenar os serviços de um projeto

> **Implementado e testado, não commitado nem publicado** (2026-11-08). Falta ver rodando de verdade (janela do
> app e web UI contra um daemon real) antes de publicar.

## O pedido

Na tela de um projeto, um botão **Reordenar serviços** abre um modal com a lista dos serviços, que se arrasta
como as variáveis de ambiente, e dois botões: **Reordenar** e **Cancelar**. Ao reordenar, os cards mudam de
posição na hora. A ordem fica **no servidor**, na especificação do projeto (o YAML que vai para o git), nunca
no cliente.

## Onde a ordem mora

`projects/<id>/project.yml` ganha o campo **`service_order`**: a lista de ids dos serviços, na ordem de
exibição (omitido quando vazio, então projetos que nunca foram reordenados continuam com o mesmo YAML).

Por que não a posição dentro de `services:`? Porque `InfraState::normalize` ordena essa lista por nome para o
YAML ter bytes estáveis (dois estados iguais geram o mesmo arquivo, sem diffs falsos no git). Uma ordem manual
precisaria desligar isso. Um campo separado preserva as duas coisas.

Regras (`ProjectInfra::services_in_order`, `store::services::reorder`):

- primeiro os ids de `service_order`, na ordem dele; os que ele não cita (criados depois) vêm em seguida, por
  data de criação — um serviço novo entra no fim;
- ids de serviços apagados são podados no `normalize`, junto com repetidos;
- `store::services::all_services` deixou de ordenar tudo por data de criação: lista projeto por projeto, e cada
  um na ordem de exibição. Snapshot, GUI e web UI só repetem essa ordem.

## O comando

`ProjectServicesReorder { project_id, service_ids }` → `Ok` (`rustploy-shared` 0.22.0).

- `service_ids` na ordem desejada; os que sobrarem ficam no fim, na ordem em que já eram mostrados;
- id de outro projeto, repetido ou projeto inexistente é erro `InvalidInput` e **nada muda**;
- nível do agente: **`operator`** (é só apresentação; não muda o que roda). Está no `required_level`, no
  `dispatch`, no manual do agente e no teste de cobertura.

## Os clientes

- **App (glacier):** botão em `shell/project_services.gv`; janela própria `services_reorder_window.gv` com a
  lista arrastável (`on_reorder`/`reorder_key`, o mesmo mecanismo das variáveis e do pré-deploy); ao gravar,
  `broadcast("services_reordered")` e o `on_broadcast` do app principal faz `Stream.refresh_now()`.
- **Web UI:** botão no cabeçalho do projeto e modal no `index.html`; arrastar (HTML5) e botões ↑/↓; estado no
  `Alpine.store('app')` (`servicesReorder`, `saveServicesReorder`).

## Testes

- Daemon: `reorder_muda_a_ordem_e_vai_ao_yaml` (ordem em `list`/`list_all`, `service_order` no `project.yml`,
  serviço novo no fim, poda ao apagar) e `reorder_mantem_no_fim_quem_nao_veio_e_recusa_id_ruim`.
- Web UI (Chrome headless): `services_reorder_modal_moves_and_cancels`.
- App: três testes Luau da janela (init numerado, arraste reescreve a ordem, JSON inválido não mexe).

## O que não foi verificado

- A janela do app aberta de verdade e o arraste com o mouse (só `glacier test`, sem janela).
- O modal da web UI com arraste real num navegador (o teste move pelo código).
- Um `ProjectServicesReorder` contra um daemon real, e o `project.yml` indo para o git e voltando num `pull`.
