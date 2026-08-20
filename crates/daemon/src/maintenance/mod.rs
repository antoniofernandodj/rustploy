//! Limpeza automática (agendada) de recursos Docker não usados — ver
//! `docs/plano-limpeza-automatica-docker.md`. Config é um singleton em
//! `daemon_settings` (não uma tabela própria); a execução reaproveita as
//! mesmas funções `*_core` que os botões manuais da aba Docker chamam
//! (`api::handlers::docker_prune`).

pub mod run;
pub mod scheduler;
