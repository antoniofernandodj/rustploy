# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

- [ ] **Toast de sucesso/erro na webui.** A GUI desktop
  (`crates/rustploy-gui`) ganhou toast ao salvar os formulários de serviço
  (`views/scripts/handlers/services.luau::with_spec`, 2026-09-11). A webui
  (`crates/daemon/webui/**`) tem o mesmo problema — `saveServiceSpec` e os
  outros handlers em `screens/*.js` só escrevem mensagem inline
  (`this.*Msg`) — mas não tem sistema de toast nenhum hoje, então é
  construir do zero (container fixo + CSS + helper `toast()` + ligar nos
  ~15+ pontos que hoje só setam `*Msg`), não só portar uma chamada
  existente. Ver cores/kind em `toasts.rs` do glacier-ui para manter os
  dois clientes consistentes.
