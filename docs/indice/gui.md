# Índice: rustploy-gui (Rust)

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> `.gvb`: `<app>`/`<screen>`/`<component>`, `props`, `imports` (`link(rel = import)`), `script`, `telas` (as `screen(name = …)` do app), `views` (valores de `@view ==`), `abas` (valores de `@tab`/`@*_tab`) e `handlers` (`on_*` e `action`).
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-gui/

### build.rs — Build script: logos dos blueprints para o release, `.desktop` no `cargo install` e recursos do `.exe…
fn main()

## rustploy-gui/src/

### embedded.rs — Assets embutidos no binário — modo standalone (só em builds de release).
fn assets() -> EmbeddedDirs — Cada pasta sob o prefixo lógico que o markup usa: `views/` (templates `.gvb`, estilos e scripts Luau…
(2 testes)

### main.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates and rendered by the pu…
fn main() -> iced::Result

## rustploy-gui/src/app/

### mod.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates and rendered by the pu…
fn run() -> iced::Result — Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.

## rustploy-gui/tests/

### app_manifest.rs — O que o Luau não alcança: o manifesto do `app(...)` (título, tamanho e fontes das janelas), a estrut…
fn boot() -> GlacierUI — Boots the engine the way `main.rs` does, but from the workspace root so the workspace-relative templ…
fn cd_ws_root() — Cd's to the workspace root (idempotent — safe alongside `boot`).
fn comments_outside(src) -> String — Remove os comentários (`//` e `/* … */`) para que "a primeira tag" seja a primeira tag de verdade: t…
(7 testes)

## rustploy-gui/tests/fixtures/

### access_key_fixture.gvb — Fixture do teste access_key.rs: tela mínima que roda o access_key.luau e deixa os resultados no cont…
<screen "fixture">
script: access_key_fixture.luau

### access_key_fixture.luau — Fixture do teste `access_key.rs` (nome diferente do módulo, senão o `require` resolve para ela mesma…
function init()

### compose_host.gvb — Fixture do teste format_service_detail.rs: tela mínima que roda o format/service_detail.luau e exibe…
<screen "fixture">
script: compose_host.luau

### compose_host.luau — Fixture do teste `format_service_detail.rs`: exercita `compose_host` e `internal_url` de `format/ser…
function init()

### saved_action.gvb — Fixture do teste busy.rs: um botão cuja ação toasta e depois ainda faz outro fetch (o "reler o servi…
<screen "fixture">
script: saved_action.luau
handlers: salvar

### saved_action.luau — Fixture do teste `busy.rs`.
function init() — Fixture do teste `busy.rs`.
function salvar()

### time.gvb — Fixture do teste format_time.rs: tela mínima que roda o format/time.luau e exibe o resultado.
<screen "fixture">
script: time.luau

### time.luau — Fixture do teste `format_time.rs`: exercita o `format/time.luau` de verdade, através do motor, e dei…
function init()

## rustploy-gui/tests/format/

### dashboard.test.luau
local com_menu(aberto) — grid_cols lê o global `ctx` (a gaveta lateral come largura da grade).
local par(nome, projeto, extras)
function test_deployments__filtra_por_servico_ou_projeto_e_formata_o_estado() — - filtra por serviço ou projeto e formata o estado
function test_cards_de_servico__status_porta_e_metricas() — - status, porta e métricas
function test_cards_de_servico__sem_metricas_o_card_mostra() — - sem métricas o card mostra —
function test_cards_de_servico__o_container_primario_e_o_live_senao_o() — - o container primário é o live; senão o primeiro em execução; +N quando há - mais
function test_cards_de_servico__o_selo_de_aquecimento_so_existe_quando_ha() — - o selo de aquecimento só existe quando há warm
function test_cards_de_servico__a_busca_casa_servico_ou_projeto() — - a busca casa serviço ou projeto
local linhas_para(n)
function test_grade__1_a_3_colunas_todas_as_fileiras_com_a_mesma_largura() — - 1 a 3 colunas, todas as fileiras com a mesma largura
function test_grade__a_ultima_fileira_e_completada_com_cards_vazios_a_grade() — - a última fileira é completada com cards vazios (a grade não tem grid)
function test_grade__o_card_vazio_carrega_os_campos_dos_dois_contratos_como() — - o card vazio carrega os campos dos dois contratos, como texto
function test_grade__a_gaveta_aberta_nunca_da_mais_colunas_que_a_fechada() — - a gaveta aberta nunca dá mais colunas que a fechada
function test_grade__sem_servicos_nao_ha_fileiras() — - sem serviços não há fileiras
function test_grade__project_rows_segue_a_mesma_regra_de_grade() — - project_rows segue a mesma regra de grade
function test_projetos__conta_servicos_e_os_que_estao_rodando_e_so_deixa() — - conta serviços e os que estão rodando, e só deixa apagar projeto vazio
function test_projetos__a_busca_olha_nome_e_descricao() — - a busca olha nome e descrição
function test_docker__docker_rows_ordena_por_nome_e_resume_a_imagem() — - docker_rows ordena por nome e resume a imagem
function test_docker__imagens_ordenadas_por_tag_sem_sha256_e_com_none_para_as() — - imagens: ordenadas por tag, sem sha256: e com <none> para as sem tag
function test_docker__imagens_em_uso_dono_e_tamanhos_crus_para_ordenar() — - imagens: em uso, dono e tamanhos crus para ordenar
function test_docker__imagens_somente_em_uso_esconde_as_orfas_e_a_busca_olha() — - imagens: 'somente em uso' esconde as órfãs, e a busca olha tags e dono
function test_docker__volumes_filtro_de_uso_tamanho_desconhecido_e_dono() — - volumes: filtro de uso, tamanho desconhecido e dono
function test_docker__redes_ordem_contagem_de_containers_e_filtro() — - redes: ordem, contagem de containers e filtro
function test_docker__containers_dono_removivel_so_parado_imagem_sem_sha256() — - containers: dono, removível só parado, imagem sem sha256:
function test_docker__containers_a_busca_olha_nome_imagem_estado_e_dono() — - containers: a busca olha nome, imagem, estado e dono
function test_ingress_portas_e_monitoramento__ingress_uma_linha_por_rota_com() — - ingress: uma linha por rota, com a porta do upstream
function test_ingress_portas_e_monitoramento__ingress_ignora_dominio_em() — - ingress ignora domínio em branco e entende o campo legado
function test_ingress_portas_e_monitoramento__portas_de_host_so_quem_tem_host() — - portas de host: só quem tem host_port, em ordem numérica
function test_ingress_portas_e_monitoramento__monitoramento_so_servicos_com() — - monitoramento: só serviços com métricas vivas, com valores crus
local info(estado, extras)
local status_dos_passos(i)
function test_motor_de_deploy__sao_oito_passos_do_fila_ao_live() — - são oito passos, do 'Fila' ao 'Live'
function test_motor_de_deploy__em_andamento_os_anteriores_feitos_o_atual() — - em andamento: os anteriores feitos, o atual marcado, os seguintes pendentes
function test_motor_de_deploy__os_quatro_estados_de_obter_caem_no_mesmo_passo() — - os quatro estados de 'Obter' caem no mesmo passo
function test_motor_de_deploy__live_stopped_e_pruning_terminam_com_tudo_feito() — - Live, Stopped e Pruning terminam com tudo feito
function test_motor_de_deploy__falha_marca_o_passo_em_que_estava_achado_no() — - falha marca o passo em que estava, achado no histórico
function test_motor_de_deploy__falha_sem_historico_nao_marca_nenhum_passo() — - falha sem histórico não marca nenhum passo como atual
function test_motor_de_deploy__estado_desconhecido_deixa_tudo_pendente() — - estado desconhecido deixa tudo pendente
function test_motor_de_deploy__detalhe_duracao_entre_transicoes_tipo_e_motivo() — - detalhe: duração entre transições, tipo e motivo
function test_motor_de_deploy__detalhe_a_transicao_corrente_mostra_o_tempo_no() — - detalhe: a transição corrente mostra o tempo no estado e fica 'info'
function test_motor_de_deploy__detalhe_timestamp_invalido_nao_da_duracao() — - detalhe: timestamp inválido não dá duração negativa
function test_motor_de_deploy__executando_agora_traz_estado_tempos_e_os() — - executando agora traz estado, tempos e os passos
function test_motor_de_deploy__a_fila_numera_de_1_e_guarda_o_id_para_cancelar() — - a fila numera de 1 e guarda o id para cancelar
function test_motor_de_deploy__o_historico_escolhe_o_icone_pelo_resultado() — - o histórico escolhe o ícone pelo resultado

### git_registry.test.luau
function test_provedor_conectado_mostra_o_login() — - provedor conectado mostra o @login
function test_provedor_sem_conta_diz_que_nao_esta_conectado() — - provedor sem conta diz que não está conectado
function test_tipo_desconhecido_aparece_como_veio_e_ausente_como_git() — - tipo desconhecido aparece como veio, e ausente como Git
function test_listas_vazias_viram_e_nao() — - listas vazias viram [] e não {}
function test_repositorios_marcam_private_public_e_branches_so_carregam_o() — - repositórios marcam private/public e branches só carregam o nome
function test_format_registry__repos_filtra_pelo_termo_e_formata_contagem_e() — - repos filtra pelo termo e formata contagem e tamanho
function test_format_registry__repos_sem_termo_traz_todos_com_defaults() — - repos sem termo traz todos, com defaults seguros
function test_format_registry__tags_encurtam_o_digest_para_12_e_guardam_o() — - tags encurtam o digest para 12 e guardam o completo
function test_format_registry__tokens_que_nunca_foram_usados_dizem_nunca() — - tokens que nunca foram usados dizem nunca
function test_format_docker_cleanup__resource_label_traduz_os_conhecidos_e() — - resource_label traduz os conhecidos e devolve o resto
function test_format_docker_cleanup__sem_execucao_anterior() — - sem execução anterior
function test_format_docker_cleanup__resumo_soma_os_bytes_e_lista_cada() — - resumo soma os bytes e lista cada recurso
function test_format_docker_cleanup__um_recurso_com_erro_avisa_e_nao_soma() — - um recurso com erro avisa e não soma contagem
function test_format_docker_cleanup__execucao_sem_resultados_mostra_so_o() — - execução sem resultados mostra só o cabeçalho

### jobs.test.luau
function test_recorrencia__sem_recorrencia_e_manual() — - sem recorrência é manual
function test_recorrencia__intervalo_diaria_e_semanal() — - intervalo, diária e semanal
function test_recorrencia__o_dia_da_semana_da_a_volta_modulo_7() — - o dia da semana dá a volta (módulo 7)
function test_recorrencia__forma_desconhecida_vira_traco() — - forma desconhecida vira traço
function test_estado__ultima_execucao_nunca_rodando_ok_falhou() — - última execução: nunca, rodando, ok, falhou
function test_estado__aquecimento_aquecendo_vence_quente_e_nil_conta_como() — - aquecimento: aquecendo vence quente, e nil conta como frio
function test_estado__servico_sem_build_nao_tem_selo() — - serviço sem build não tem selo
local resumo(sobrescrita)
function test_tela_schedules__uma_linha_por_job_com_o_total() — - uma linha por job, com o total
function test_tela_schedules__o_dono_mostra_projeto_servico_gatilho_ou_so_o() — - o dono mostra projeto / serviço gatilho, ou só o projeto quando autônomo
function test_tela_schedules__o_nome_comprido_e_truncado_na_exibicao_e() — - o nome comprido é truncado na exibição e guardado inteiro
function test_tela_schedules__running_reflete_o_servidor_ou_o_clique_em_voo() — - running reflete o servidor OU o clique em voo
function test_tela_schedules__proxima_execucao_e_aquecimento() — - próxima execução e aquecimento
function test_tela_schedules__sem_next_run_at_nem_aquecimento_os_campos_tem() — - sem next_run_at nem aquecimento, os campos têm padrão seguro
function test_tela_schedules__job_que_nunca_rodou() — - job que nunca rodou
function test_tela_schedules__sem_jobs_da_e_zero() — - sem jobs dá [] e zero
function test_fila_de_pre_deploy__a_fila_respeita_a_ordem_configurada_e() — - a fila respeita a ordem configurada e resolve o nome
function test_fila_de_pre_deploy__um_job_apagado_por_fora_aparece_como() — - um job apagado por fora aparece como removido, para dar para tirá-lo da - fila
function test_fila_de_pre_deploy__o_id_legado_vale_quando_a_fila_nova_esta() — - o id legado vale quando a fila nova está vazia
function test_fila_de_pre_deploy__os_disponiveis_sao_os_que_ainda_nao_estao() — - os disponíveis são os que ainda não estão na fila
function test_fila_de_pre_deploy__fila_e_disponiveis_juntos_cobrem_todos_os() — - fila e disponíveis juntos cobrem todos os jobs, sem repetir
function test_fila_de_pre_deploy__sem_jobs_da() — - sem jobs dá []
function test_historico__cada_execucao_com_seu_estado_codigo_de_saida_e() — - cada execução com seu estado, código de saída e horários
function test_historico__sem_execucoes_da() — - sem execuções dá []

### service_detail.test.luau
local postgres_vars()
function test_origem_e_healthcheck__source_summary_parts_de_cada_origem() — - source_summary_parts de cada origem
function test_origem_e_healthcheck__source_summary_parts_de_origem_ausente() — - source_summary_parts de origem ausente
function test_origem_e_healthcheck__healthcheck_desligado() — - healthcheck desligado
function test_origem_e_healthcheck__healthcheck_http_mostra_caminho_e_status() — - healthcheck HTTP mostra caminho e status esperado
function test_origem_e_healthcheck__healthcheck_tcp_docker_e_tipo() — - healthcheck TCP, Docker e tipo desconhecido
function test_containers__cor_por_estado_e_id_encurtado() — - cor por estado e id encurtado
function test_containers__sem_containers_da() — - sem containers dá []
function test_variaveis_de_ambiente__segredo_aparece_por_referencia_com() — - segredo aparece por referência, com cadeado, e se distingue do texto igual
function test_variaveis_de_ambiente__a_chave_longa_trunca_so_na_exibicao() — - a chave longa trunca só na exibição
function test_variaveis_de_ambiente__comentarios_voltam_para_junto_da_chave_a() — - comentários voltam para junto da chave a que estavam ancorados
function test_variaveis_de_ambiente__o_env_sai_com_segredo_entre_secret_e() — - o .env sai com segredo entre <secret:> e comentários nos lugares
function test_variaveis_de_ambiente__o_env_faz_ida_e_volta_com_o_parser_de() — - o .env faz ida e volta com o parser de .env
function test_variaveis_de_ambiente__vazio_da_e_string_vazia() — - vazio dá [] e string vazia
function test_variaveis_de_ambiente__secret_rows_so_carrega_nomes() — - secret_rows só carrega nomes
function test_logs__stderr_e_vermelho_ansi_some_e_a_hora_e_formatada() — - stderr é vermelho, ANSI some e a hora é formatada
function test_logs__so_a_cauda_de_500_linhas_e_materializada() — - só a cauda de 500 linhas é materializada
function test_logs__linha_absurda_e_cortada_em_2000_caracteres() — - linha absurda é cortada em 2000 caracteres
function test_logs__build_logs_json_e_o_mesmo_formatador() — - build_logs_json é o mesmo formatador
function test_logs__join_log_lines_junta_tudo_sem_o_teto_de_linhas() — - join_log_lines junta tudo, sem o teto de linhas
function test_deployments_e_dominios__linha_de_deployment_id_estado_duracao_e() — - linha de deployment: id, estado, duração e se pode apagar
function test_deployments_e_dominios__todo_estado_terminal_pode_ser_apagado() — - todo estado terminal pode ser apagado, nenhum em andamento
function test_deployments_e_dominios__a_rota_sem_porta_propria_herda_a_do() — - a rota sem porta própria herda a do serviço
function test_deployments_e_dominios__sem_dominio_da() — - sem domínio dá []
function test_nomes_e_hosts__safe_name_minusculas_nao_alfanumerico_vira() — - safe_name: minúsculas, não-alfanumérico vira _ colapsado e sem bordas
function test_nomes_e_hosts__compose_host_ingress_service_declarado_vence() — - compose_host: ingress_service declarado vence
function test_nomes_e_hosts__compose_host_primeira_chave_de_services() — - compose_host: primeira chave de services:
function test_nomes_e_hosts__compose_host_sem_services_ou_com_o_bloco_vazio() — - compose_host: sem services: ou com o bloco vazio dá nil
function test_url_interna__esquema_por_tipo_de_banco_e_host_rp_nome() — - esquema por tipo de banco e host rp_<nome>
function test_url_interna__servico_comum_e_http_e_kafka_e_host_porta_puro() — - serviço comum é http, e kafka é host:porta puro
function test_url_interna__servico_compose_usa_o_nome_do_yaml_nao_rp_nome() — - serviço compose usa o nome do YAML, não rp_<nome>
function test_url_externa__dominio_http_tem_prioridade_e_respeita_o_tls() — - domínio HTTP tem prioridade e respeita o TLS
function test_url_externa__dominio_so_com_espacos_nao_conta() — - domínio só com espaços não conta
function test_url_externa__sem_dominio_nem_porta_publicada_nao_ha_url() — - sem domínio nem porta publicada, não há URL
function test_url_externa__postgres_vira_string_jdbc_com_as_credenciais_das() — - postgres vira string JDBC com as credenciais das env vars
function test_url_externa__postgres_sem_credenciais_devolve_a_base_intacta() — - postgres sem credenciais devolve a base intacta
function test_url_externa__mysql_e_mariadb_usuario_senha_e_banco_no_userinfo() — - mysql e mariadb: usuário, senha e banco no userinfo
function test_url_externa__redis_so_tem_senha_userinfo_comeca_em() — - redis só tem senha: userinfo começa em :
function test_url_externa__mongodb_leva_barra_final_e_authsource_admin_quando() — - mongodb leva barra final e authSource=admin quando há usuário
function test_url_externa__rabbitmq_e_nats() — - rabbitmq e nats
function test_url_externa__sem_esquema_de_banco_kafka_servico_comum_e_host() — - sem esquema de banco (kafka, serviço comum) é host:porta puro
function test_url_externa__senha_com_caracteres_reservados_e_percent_encoded() — - senha com caracteres reservados é percent-encoded
function test_url_externa__segredo_referencia_nao_vira_credencial() — - segredo (referência) não vira credencial
function test_url_externa__o_host_sai_do_api_url_ou_host_quando_ele_falta() — - o host sai do api_url, ou <host> quando ele falta

### time.test.luau
function test_duracoes__format_seconds() — - format_seconds
function test_duracoes__format_seconds_aceita_string_fracao_e_lixo() — - format_seconds aceita string, fração e lixo
function test_duracoes__format_uptime_escolhe_a_maior_unidade() — - format_uptime escolhe a maior unidade
function test_duracoes__format_bytes() — - format_bytes
function test_duracoes__format_duration_subtrai_instantes_mesmo_com_fuso() — - format_duration subtrai instantes, mesmo com fuso explícito
function test_duracoes__format_duration_nunca_e_negativa() — - format_duration nunca é negativa
function test_duracoes__format_duration_sem_started_at_valido_da_0s() — - format_duration sem started_at válido dá 0s
function test_hora_local__aceitam_nil_e_devolvem_vazio_finished_at_ainda_nao() — - aceitam nil e devolvem vazio (finished_at ainda não preenchido)
function test_hora_local__a_forma_do_texto_e_fixa_qualquer_que_seja_o_fuso_da() — - a forma do texto é fixa, qualquer que seja o fuso da máquina
function test_hora_local__os_minutos_e_segundos_nao_dependem_do_fuso_inteiro() — - os minutos e segundos não dependem do fuso inteiro
function test_ponte_do_timeedit__hour_minute_join_formata_com_zeros() — - hour_minute_join formata com zeros
function test_ponte_do_timeedit__hour_minute_join_satura_fora_da_faixa_e() — - hour_minute_join satura fora da faixa e tolera lixo
function test_ponte_do_timeedit__hour_minute_split_e_o_inverso_de_join() — - hour_minute_split é o inverso de join
function test_ponte_do_timeedit__hour_minute_split_de_chave_vazia_malformada() — - hour_minute_split de chave vazia, malformada ou fora da faixa vira 0h00
function test_dias_da_semana__weekdays_e_uma_lista_de_sete_id_label_com() — - WEEKDAYS é uma lista de sete {id,label} com segunda = 0

### util.test.luau
function test_busca__termo_vazio_ou_nil_sempre_casa_a_busca_e_filtro_nao() — - termo vazio ou nil sempre casa (a busca é filtro, não requisito)
function test_busca__casa_por_trecho_sem_diferenciar_caixa_do_campo() — - casa por trecho, sem diferenciar caixa do campo
function test_busca__o_termo_e_literal_nao_padrao_lua() — - o termo é literal, não padrão Lua
function test_busca__campos_que_nao_sao_texto_sao_ignorados() — - campos que não são texto são ignorados
function test_texto__ellipsis_nao_mexe_no_que_cabe() — - ellipsis não mexe no que cabe
function test_texto__ellipsis_corta_em_caracteres_nao_em_bytes_e_anexa() — - ellipsis corta em caracteres, não em bytes, e anexa …
function test_texto__short_reason_fica_com_a_primeira_linha_nao_vazia_aparada() — - short_reason fica com a primeira linha não vazia, aparada
function test_texto__short_reason_trunca_no_limite_pedido_e_tem_padrao_de_140() — - short_reason trunca no limite pedido e tem padrão de 140
function test_texto__short_reason_devolve_vazio_sem_motivo() — - short_reason devolve vazio sem motivo
function test_texto__strip_ansi_tira_cor_cursor_osc_cr_e_nul() — - strip_ansi tira cor, cursor, OSC, CR e NUL
function test_texto__strip_ansi_nao_mexe_em_texto_limpo() — - strip_ansi não mexe em texto limpo
function test_arrays__encode_array_forca_quando_vazio_tabela_vazia_viraria() — - encode_array força [] quando vazio (tabela vazia viraria {})
function test_arrays__encode_array_codifica_a_lista_quando_ha_itens() — - encode_array codifica a lista quando há itens
function test_estados__status_label_color_e_status_kind_concordam_em_todos_os() — - status_label_color e status_kind concordam em todos os estados
function test_estados__status_com_erro_tabela_ou_desconhecido_cai_em_error() — - status com erro (tabela) ou desconhecido cai em Error/bad
function test_estados__estado_de_deploy_rotulo_cor_e_kind() — - estado de deploy: rótulo, cor e kind
function test_estados__in_use_kind() — - in_use_kind
function test_estados__container_state_normaliza_a_caixa_e_classifica() — - container_state normaliza a caixa e classifica
function test_estados__container_state_parado_ou_desconhecido_e_muted_vazio() — - container_state: parado ou desconhecido é muted; vazio vira exited
function test_estados__container_stopped_so_ativo_running_restarting_paused() — - container_stopped: só ativo (running/restarting/paused) não é removível
function test_dominio_do_servico__source_summary_de_cada_origem() — - source_summary de cada origem
function test_dominio_do_servico__source_summary_de_origem_ausente_ou() — - source_summary de origem ausente ou desconhecida é —
function test_dominio_do_servico__domain_routes_usa_a_lista_nova_e_converte() — - domain_routes usa a lista nova e converte tls para booleano
function test_dominio_do_servico__domain_routes_cai_no_domain_tls_enabled() — - domain_routes cai no domain/tls_enabled legado
function test_dominio_do_servico__a_lista_nova_vence_o_legado_e_vazio_nao() — - a lista nova vence o legado, e vazio não gera rota
function test_dominio_do_servico__pre_deploy_checks_a_fila_nova_vence_o_id() — - pre_deploy_checks: a fila nova vence o id legado
function test_dominio_do_servico__service_pair_list_normaliza_o_snapshot() — - service_pair_list normaliza o snapshot
function test_orcamento_de_colunas__segue_os_breakpoints_560_900_da_largura() — - segue os breakpoints 560/900 da largura da janela

## rustploy-gui/tests/handlers/

### connection.test.luau — Sessão e login (handlers/connection.luau).
local nova(ctx_inicial)
function test_init__a_janela_nasce_no_login_na_visao_de_deployments() — - a janela nasce no login, na visão de deployments, desconectada
function test_init__o_init_nao_pisa_no_que_ja_estava_no_contexto() — - o init não pisa no que já estava no contexto
function test_init__as_listas_nascem_como_e_nao_nil_o_for_each_nao_pode_ler() — - as listas nascem como [] e não nil (o for-each não pode ler literal)
function test_init__os_contadores_nascem_e_nao_0_0_afirmaria_vazio_antes_do() — - os contadores nascem … e não 0: 0 afirmaria vazio antes do primeiro - snapshot
function test_init__o_contador_ja_publicado_nao_e_trocado_por() — - o contador já publicado não é trocado por …
function test_init__as_colecoes_fixas_dos_widgets_sao_json_de_id_label() — - as coleções fixas dos widgets são JSON de {id,label}
function test_init__o_init_aponta_o_servidor_da_chave_semeada() — - o init aponta o servidor da chave semeada
function test_init__os_checkboxes_de_iac_nascem_desmarcados() — - os checkboxes de IaC nascem desmarcados
function test_campo_do_token__digitar_uma_chave_valida_mostra_o_servidor_e() — - digitar uma chave válida mostra o servidor e limpa o erro
function test_campo_do_token__uma_chave_invalida_nao_mostra_servidor_nenhum() — - uma chave inválida não mostra servidor nenhum
function test_campo_do_token__escolher_um_servidor_salvo_preenche_o_campo_do() — - escolher um servidor salvo preenche o campo do mesmo jeito
function test_campo_do_token__a_lista_de_servidores_salvos_e_um_array_json() — - a lista de servidores salvos é um array JSON
function test_connect_so_o_que_falha_antes_da_rede__sem_token_pede_para_colar() — - sem token pede para colar
function test_connect_so_o_que_falha_antes_da_rede__o_token_da_api_no_lugar() — - o token da API no lugar do de conexão é explicado
function test_connect_so_o_que_falha_antes_da_rede__chave_corrompida_avisa_e() — - chave corrompida avisa e não muda de tela
function test_disconnect__apaga_o_contexto_da_sessao_e_volta_ao_login() — - apaga o contexto da sessão e volta ao login
function test_disconnect__nao_enumera_chaves_o_init_semeia_os_defaults_de() — - não enumera chaves: o init semeia os defaults de novo, como no boot

### misc.test.luau — Handlers que só mexem no ctx, ou que não fazem nada sem conexão: deploy_queue, secrets e bundle.
local nova(ctx_inicial)
function test_deploy_queue_sem_conexao__cancelar_promover_reordenar_e_pausar() — - cancelar, promover, reordenar e pausar são inertes
function test_deploy_queue_sem_conexao__id_vazio_tambem_nao_faz_nada() — - id vazio também não faz nada
function test_deploy_queue_detalhe_do_deploy__sem_um_deploy_ativo_com_esse_id() — O painel só faz sentido para um deploy ATIVO: open_detail marca o serviço e logo em seguida refresh_…
function test_deploy_queue_detalhe_do_deploy__fechar_limpa_o_id_e_desmarca() — - fechar limpa o id e desmarca aberto
function test_secrets__secret_use_arma_o_form_em_modo_secret_com_o_nome() — - secret_use arma o form em modo secret com o nome escolhido
function test_secrets__secret_use_nao_pisa_numa_chave_que_o_usuario_ja() — - secret_use não pisa numa chave que o usuário já digitou
function test_secrets__secret_use_trata_chave_so_de_espacos_como_vazia() — - secret_use trata chave só de espaços como vazia
function test_secrets__o_interruptor_de_secret_alterna_e_limpa_o_valor() — - o interruptor de secret alterna e limpa o valor digitado
function test_secrets__a_falha_de_validacao_so_avisa() — - a falha de validação só avisa
function test_secrets__adicionar_sem_projeto_ou_sem_conexao_nao_faz_nada() — - adicionar sem projeto ou sem conexão não faz nada
function test_secrets__apagar_sem_projeto_ou_com_nome_vazio_nao_pergunta_nem() — - apagar sem projeto ou com nome vazio não pergunta nem faz nada
function test_bundle_exportar_servico__sem_conexao_ou_sem_servico_escolhido() — - sem conexão ou sem serviço escolhido não abre janela nem avisa

### nav.test.luau — Navegação da sidebar, abas e busca (handlers/nav.luau), sem conexão.
local nova()
function test_visoes__a_sidebar_troca_a_visao_e_limpa_o_servico_selecionado() — - a sidebar troca a visão e limpa o serviço selecionado
function test_visoes__projetos_tambem_limpa_o_projeto_selecionado() — - projetos também limpa o projeto selecionado
function test_visoes__o_deploy_engine_abre_na_aba_da_fila_sem_pisar_numa_aba() — - o Deploy Engine abre na aba da fila, sem pisar numa aba já escolhida
function test_visoes__as_configuracoes_abrem_na_aba_web_e_semeiam_o_provedor() — - as configurações abrem na aba web e semeiam o provedor Git sem pisar no - escolhido
function test_visoes__nav_generico_vai_para_a_visao_pedida_e_ignora_valor() — - nav genérico vai para a visão pedida e ignora valor vazio
function test_abas_e_campos__cada_aba_grava_a_sua_chave() — - cada aba grava a sua chave
function test_abas_e_campos__o_setter_generico_field_grava_a_chave_com_o() — - o setter genérico field grava a chave com o valor
function test_abas_e_campos__a_busca_guarda_o_texto_como_veio_e_nao_quebra() — - a busca guarda o texto como veio e não quebra sem snapshot

## rustploy-gui/tests/lib/

### access_key.test.luau
function test_address_of__a_porta_sempre_vai_mesmo_a_padrao() — - a porta sempre vai, mesmo a padrão
function test_address_of__caminho_query_e_fragmento_nao_entram() — - caminho, query e fragmento não entram
function test_address_of__sem_esquema_vale_como_https() — - sem esquema vale como https
function test_address_of__ipv6_entre_colchetes_os_do_endereco_nao_sao_porta() — - IPv6 entre colchetes: os : do endereço não são porta
function test_encode__e_o_base64_padrao_de_host_porta_token() — Os valores esperados saem do `base64.b64encode` do Python, que é o formato que o daemon escreve no c…
function test_encode__token_vazio_api_aberta_ainda_gera_uma_chave() — - token vazio (API aberta) ainda gera uma chave
function test_encode__o_separador_e() — - o separador é ::
function test_decode__le_o_que_o_python_gera() — - lê o que o Python gera
function test_decode__encode_e_decode_sao_inversos() — - encode e decode são inversos
function test_decode__aceita_o_alfabeto_url_safe_e_a_falta_de_padding() — - aceita o alfabeto url-safe e a falta de padding
function test_decode__apara_espacos_em_volta_da_chave_colada() — - apara espaços em volta da chave colada
function test_decode__um_esquema_posto_a_mao_e_pulado() — - um esquema posto à mão é pulado
function test_decode__endereco_e_token_com_espacos_sao_aparados() — - endereço e token com espaços são aparados
function test_decode__token_vazio_vale_daemon_sem_api_token() — - token vazio vale (daemon sem [api].token)
function test_decode__vazio_pede_para_colar() — - vazio pede para colar
function test_decode__caractere_fora_do_alfabeto_ou_comprimento_impossivel() — - caractere fora do alfabeto ou comprimento impossível = corrompido
function test_decode__o_token_da_api_colado_no_lugar_do_de_conexao_e() — - o token da API colado no lugar do de conexão é reconhecido
function test_decode__bytes_que_nao_sao_texto_tambem_caem_nesse_aviso() — - bytes que não são texto também caem nesse aviso
function test_decode__endereco_vazio_e_corrompido() — - endereço vazio é corrompido
function test_hint__mostra_o_servidor_e_as_pontas_da_chave() — - mostra o servidor e as pontas da chave

### busy.test.luau — busy.luau envolve handlers globais numa trava: enquanto um roda, a chave `busy_<nome>` do ctx fica "…
local ambiente()
function test_key_monta_o_nome_da_chave() — - key monta o nome da chave
function test_a_chave_fica_true_durante_o_handler_e_false_depois() — - a chave fica true durante o handler e false depois
function test_repassa_argumentos_e_valores_de_retorno() — - repassa argumentos e valores de retorno
function test_um_segundo_disparo_durante_a_execucao_e_descartado() — - um segundo disparo durante a execução é descartado
function test_depois_de_terminar_o_handler_pode_rodar_de_novo() — - depois de terminar, o handler pode rodar de novo
function test_erro_no_handler_se_propaga_e_mesmo_assim_solta_a_trava() — - erro no handler se propaga e mesmo assim solta a trava
function test_toast_solta_a_trava_de_quem_o_chamou_a_acao_ja_deu_o_retorno() — - toast solta a trava de quem o chamou (a ação já deu o retorno)
function test_o_toast_original_continua_sendo_chamado_com_as_opcoes() — - o toast original continua sendo chamado com as opções
function test_init_semeia_false_em_todas_as_chaves_e_chama_o_init_original() — - init semeia false em todas as chaves e chama o init original
function test_init_funciona_mesmo_sem_um_init_original() — - init funciona mesmo sem um init original
function test_nome_sem_funcao_global_nao_ganha_trava_nem_semente() — - nome sem função global não ganha trava nem semente

### busy_actions.test.luau — A lista de ações com trava é mantida por `tools/busy_actions.py --check` contra os templates; este t…
function test_nao_esta_vazia() — - não está vazia
function test_so_tem_nomes_de_funcao_validos_em_luau() — - só tem nomes de função válidos em Luau
function test_nao_repete_nome_a_segunda_trava_envolveria_a_primeira() — - não repete nome (a segunda trava envolveria a primeira)
function test_esta_em_ordem_alfabetica_para_o_diff_do_check_ser_legivel() — - está em ordem alfabética, para o diff do --check ser legível

### helpers.test.luau
function test_trim__apara_as_pontas_e_aceita_nil_e_nao_texto() — - apara as pontas e aceita nil e não-texto
function test_referencia_a_segredo__aceita_secret_x_e_secret_x() — - aceita <secret:x> e secret:x
function test_referencia_a_segredo__o_que_nao_e_referencia_devolve_nil() — - o que não é referência devolve nil
function test_env__separa_chaves_valores_e_segredos() — - separa chaves, valores e segredos
function test_env__o_valor_pode_conter() — - o valor pode conter =
function test_env__comentarios_ficam_ligados_a_chave_que_vem_depois() — - comentários ficam ligados à chave que vem depois
function test_env__comentario_no_fim_nao_tem_chave_seguinte() — - comentário no fim não tem chave seguinte
function test_env__linhas_vazias_e_sem_sao_ignoradas() — - linhas vazias e sem = são ignoradas
function test_env__entrada_vazia_ou_nil_da_listas_vazias_que_codificam_como() — - entrada vazia ou nil dá listas vazias que codificam como array
function test_env__aceita_final_de_linha_sem_n() — - aceita final de linha sem \\n
function test_urls__oauth_redirect_uri_monta_o_callback_do_provedor() — - oauth_redirect_uri monta o callback do provedor
function test_urls__provedor_desconhecido_cai_em_gitea_e_base_vazia_avisa() — - provedor desconhecido cai em gitea, e base vazia avisa
function test_urls__looks_like_git_url_reconhece_os_esquemas_e_o_sufixo_git() — - looks_like_git_url reconhece os esquemas e o sufixo .git
function test_urls__normalize_url_fixa_o_esquema_e_descarta_caminho_query_e() — - normalize_url fixa o esquema e descarta caminho, query e fragmento
function test_urls__normalize_url_recusa_vazio_e_esquema_que_nao_e_http_s() — - normalize_url recusa vazio e esquema que não é http(s)
function test_booleanos__is_true_aceita_boolean_e_texto() — - is_true aceita o boolean (checkbox/toggle) e o texto "true"; o resto é falso
function test_booleanos__is_false_nao_confunde_ausente_com_falso() — - is_false só é verdadeiro para o false explícito; ausente não é falso (padrão ligado)

### manifest_zip.test.luau
function test_ida_e_volta_preserva_yaml_e_toml() — - ida e volta preserva yaml e toml
function test_recusa_arquivo_extra() — - recusa arquivo extra
function test_recusa_subpasta() — - recusa subpasta
function test_recusa_sem_toml() — - recusa sem toml
function test_recusa_dois_yaml() — - recusa dois yaml
function test_zip_inexistente_da_erro() — - zip inexistente dá erro

## rustploy-gui/tests/tela/

### missing.test.luau — Toda chave que uma tela LÊ tem de ser publicada por alguém.
local conjunto(...)
local confere(tela, ...)
local app(visao)
function test_login__a_tela_de_login_so_espera_o_erro_opcional_e_o_estado_da() — - a tela de login só espera o erro opcional e o estado da janela
function test_shell_cada_visao__deployments() — - deployments
function test_shell_cada_visao__projects() — - projects
function test_shell_cada_visao__project_services() — - project_services
function test_shell_cada_visao__service() — - service
function test_shell_cada_visao__running() — - running
function test_shell_cada_visao__monitoring() — - monitoring
function test_shell_cada_visao__ingress() — - ingress
function test_shell_cada_visao__deploy_engine() — - deploy_engine
function test_shell_cada_visao__docker() — - docker
function test_shell_cada_visao__settings() — - settings
function test_shell_cada_visao__schedules() — - schedules
function test_shell_cada_visao__support() — - support
local janela(caminho, ctx_inicial)
function test_janelas_auxiliares__logs_ao_vivo() — - logs ao vivo
function test_janelas_auxiliares__novo_job() — - novo job
function test_janelas_auxiliares__novo_servico_com_o_projeto_alvo_semeado() — - novo serviço (com o projeto-alvo semeado pelo open_window)
function test_janelas_auxiliares__novo_projeto() — - novo projeto
function test_janelas_auxiliares__novo_token_do_registry() — - novo token do registry
function test_janelas_auxiliares__exportar_servico() — - exportar serviço

### render.test.luau — O que a tela MOSTRA, não só o estado que os handlers escrevem.
local app(visao, extra) — O init do app pré-preencheria `access_key` com o servidor salvo da máquina.
local rende(tela) — Avalia a tela com o ctx de agora; lança se o markup estiver quebrado.
local seta(tela, dados)
local tem_trecho(lista, trecho)
local contagem(lista, esperados)
function test_o_app_todo_carrega() — - o app e todos os imports carregam (uma view quebrada, mesmo não usada, - falha aqui)
function test_cada_janela_auxiliar_carrega() — - cada janela auxiliar carrega por conta própria
function test_login_renderiza() — - a tela de login aparece sem sessão
function test_visoes_do_shell() — - todas as visões do shell avaliam
local janela(caminho, dados)
function test_janela_novo_projeto_renderiza() — - janela Novo projeto
function test_janela_novo_job_passos_e_recorrencias() — - janela Novo job, em cada passo e em cada tipo de recorrência
function test_janela_de_logs_renderiza() — - janela de logs ao vivo
local wizard(dados)
function test_janela_novo_servico_passos() — - wizard Novo serviço, em cada passo
function test_janela_novo_servico_relatorio_de_importacao() — - passo Importar, na tela do relatório (todas as listas e ramos preenchidos)
function test_janela_exportar_servico_renderiza() — - janela Exportar serviço: renderiza com o plano semeado
function test_deploy_engine_abas() — - as três abas do Deploy Engine, com fila, execução (stepper + popup) e - histórico
function test_ingress_portas_do_host() — - portas do host na tela de ingress
local docker()
function test_docker_cada_aba() — - cada aba do Docker avalia
function test_docker_a_celula_da_linha_e_avaliada() — - a célula da tabela sai avaliada com a linha (`@c.name`), não só o cabeçalho
function test_docker_registry_com_repositorio_selecionado() — - o registry com um repositório selecionado lista as tags
function test_agenda_job_parado() — - job parado: estado frio e todas as ações no painel — menos cancelar O painel "Ações do job" mora d…
function test_agenda_job_rodando() — - job rodando: quente, cancelável, e não oferece rodar de novo
local projeto()
function test_projeto_cada_aba() — - cada aba do projeto avalia
function test_projeto_aba_jobs_mostra_estado_e_acoes() — - a aba Jobs do projeto mostra o estado e as ações
function test_projeto_env_em_modo_secret_e_sem_secrets() — - variáveis em modo secret, e sem nenhum secret cadastrado
function test_configuracoes_git() — - Git: provedor OAuth e token
function test_configuracoes_web() — - Web Server
function test_configuracoes_iac() — - Infra as Code: manifesto exportado, variáveis faltando e relatório
function test_configuracoes_manutencao() — - Manutenção: limpeza do Docker em cada recorrência
local servico(aba)
function test_servico_cada_aba() — - cada aba da tela de serviço avalia
function test_servico_geral_por_origem() — - aba Geral: origem Git, zip e compose
function test_servico_webhook() — - aba Deployments: webhook sem token e em serviço compose
function test_so_a_tela_ativa_e_avaliada() — - só a tela ativa é avaliada: o login não vaza para dentro do shell
function test_grades_de_cards_com_spread() — - as grades de cards (projetos e serviços) passam o item inteiro (`spread`)
function test_em_execucao_cards_aninhados() — - a tela Em execução: o each aninhado (projeto -> serviços) chega aos cards
local nav_projects(tela)
function test_nav_projects_fica_aceso_nas_subtelas() — - o item Projects da sidebar fica aceso nas sub-telas (não só em "projects") Foi um bug real: o nav_…
function test_nav_item_aceso_recolore_os_filhos_pelo_combinador() — - o item aceso recolore ícone e rótulo pelo combinador `.nav-item--active > - …` Se a regra composta…
function test_sidebar_e_uma_gaveta_ligada_a_chave_menu() — - a sidebar é uma <drawer> ligada à chave `menu`
function test_acoes_do_servico_viram_icones_quando_estreito() — - as ações do serviço viram ícones quando a janela estreita
function test_acao_em_andamento_trava_o_botao() — - o botão de uma ação suspensa num fetch fica travado até ela terminar O init semeia `busy_<ação>` e…

### running.test.luau
local service(overrides)
local dashboard(services)
function test_mascara_o_token_do_webhook_por_padrao() — - mascara o token do webhook por padrão
function test_mostra_o_webhook_inteiro_quando_revelado() — - mostra o webhook inteiro quando revelado
function test_lista_os_pre_deploy_checks_na_ordem() — - lista os pré-deploy checks na ordem
function test_banco_mostra_o_motor_e_a_porta() — - banco mostra o motor e a porta
function test_servico_sem_webhook_avisa() — - serviço sem webhook avisa
function test_a_busca_filtra_servicos_e_some_com_projeto_sem_resultado() — - a busca filtra serviços e some com projeto sem resultado

## rustploy-gui/tests/windows/

### new_job.test.luau — Janela "Novo job" (scripts/new_job_window.luau): passos do assistente, modo edição e a validação do …
local nova(ctx_inicial)
local conectada(campos) — `api_url` semeado: sem ele new_job_create para antes de validar qualquer campo.
function test_novo_job_init__abre_escolhendo_o_projeto_com_o_formulario() — - abre escolhendo o projeto, com o formulário zerado
function test_novo_job_init__os_padroes_da_recorrencia_manual_a_cada_6h_03_00() — - os padrões da recorrência: manual, a cada 6h, 03:00, segunda
function test_novo_job_init__a_fonte_do_compose_comeca_em_colado_com_o() — - a fonte do compose começa em colado, com o caminho padrão
function test_novo_job_init__os_catalogos_semeados_pelo_pai_ficam_a() — - os catálogos semeados pelo pai ficam à disposição
function test_novo_job_init__sem_catalogo_os_projetos_nascem() — - sem catálogo os projetos nascem []
function test_novo_job_init__os_dias_da_semana_vem_do_formatador() — - os dias da semana vêm do formatador
local editando(job)
function test_novo_job_modo_edicao__pula_direto_para_o_formulario_e_preenche() — - pula direto para o formulário e preenche o job
function test_novo_job_modo_edicao__job_pausado_vem_desmarcado() — - job pausado vem desmarcado
function test_novo_job_modo_edicao__recorrencia_de_intervalo() — - recorrência de intervalo
function test_novo_job_modo_edicao__recorrencia_diaria_vira_hh_mm() — - recorrência diária vira HH:MM
function test_novo_job_modo_edicao__recorrencia_semanal_guarda_o_dia_e_o() — - recorrência semanal guarda o dia e o horário
function test_novo_job_modo_edicao__sem_recorrencia_ou_desconhecida_e_manual() — - sem recorrência (ou desconhecida) é manual com os padrões
function test_novo_job_modo_edicao__job_com_fonte_git_troca_a_fonte_e_os() — - job com fonte git troca a fonte e os campos do picker
function test_novo_job_modo_edicao__git_sem_compose_path_usa_o_padrao() — - git sem compose_path usa o padrão
function test_novo_job_modo_edicao__as_variaveis_do_job_voltam_como_env_com() — - as variáveis do job voltam como .env, com segredo e comentário
function test_novo_job_modo_edicao__json_de_edicao_ilegivel_e_ignorado_abre() — - JSON de edição ilegível é ignorado: abre como job novo
local com_catalogo()
function test_novo_job_passos__escolher_o_projeto_filtra_os_servicos_dele_e() — - escolher o projeto filtra os serviços dele e avança
function test_novo_job_passos__outro_projeto_outros_servicos() — - outro projeto, outros serviços
function test_novo_job_passos__projeto_desconhecido_nao_acha_nome_nem_servico() — - projeto desconhecido não acha nome nem serviço, mas o passo avança
function test_novo_job_passos__escolher_o_servico_gatilho_guarda_o_nome_e_vai() — - escolher o serviço gatilho guarda o nome e vai ao formulário
function test_novo_job_passos__job_autonomo_sem_servico_gatilho() — - job autônomo: sem serviço gatilho
function test_novo_job_passos__voltar_recua_um_passo_por_vez() — - voltar recua um passo por vez
function test_novo_job_passos__o_tipo_de_recorrencia_e_o_setter_generico() — - o tipo de recorrência e o setter genérico
function test_novo_job_fonte_git__trocar_a_fonte_sem_conexao_so_grava_a() — - trocar a fonte sem conexão só grava a escolha
function test_novo_job_fonte_git__escolher_a_conta_zera_repositorio_repos_e() — - escolher a conta zera repositório, repos e branches
function test_novo_job_fonte_git__escolher_o_repositorio_adota_a_branch() — - escolher o repositório adota a branch padrão dele
function test_novo_job_fonte_git__repositorio_sem_branch_padrao_deixa_a() — - repositório sem branch padrão deixa a branch como estava
local erro_ao_criar(campos, fonte)
function test_novo_job_criar_so_ate_a_validacao_recusar__sem_conexao_manda() — - sem conexão manda reabrir a janela
function test_novo_job_criar_so_ate_a_validacao_recusar__o_nome_e_obrigatorio() — - o nome é obrigatório, mesmo só de espaços
function test_novo_job_criar_so_ate_a_validacao_recusar__o_main_service_e() — - o main_service é obrigatório
function test_novo_job_criar_so_ate_a_validacao_recusar__fonte_compose_pede_o() — - fonte compose pede o docker-compose colado
function test_novo_job_criar_so_ate_a_validacao_recusar__fonte_git_pede() — - fonte git pede repositório e branch
function test_novo_job_criar_so_ate_a_validacao_recusar__git_com_repositorio() — - git com repositório que não está na lista pede para selecionar de novo
function test_novo_job_criar_so_ate_a_validacao_recusar__o_erro_some_quando_o() — - o erro some quando o formulário passa a validar (limpa antes de ir à rede)
function test_novo_job_criar_so_ate_a_validacao_recusar__o_formulario() — - o formulário recusado não deixa o botão travado

### windows.test.luau — Scripts das janelas auxiliares (cada uma é um motor isolado, aberto por open_window com `data`).
local janela(caminho, ctx_inicial)
local exportar(plano)
local linhas(tela)
function test_janela_exportar_servico__o_init_resume_o_plano() — - o init resume o plano
function test_janela_exportar_servico__as_variaveis_do_projeto_nascem() — - as variáveis do projeto nascem marcadas só quando o serviço as cita
function test_janela_exportar_servico__as_etiquetas_dizem_se_e_secret_e_se_o() — - as etiquetas dizem se é secret e se o serviço a cita
function test_janela_exportar_servico__alternar_uma_variavel_inverte_so_ela() — - alternar uma variável inverte só ela
function test_janela_exportar_servico__todas_nenhuma_e_sugeridas() — - todas, nenhuma e sugeridas
function test_janela_exportar_servico__o_plano_bloqueado_repassa_o_motivo() — - o plano bloqueado repassa o motivo
function test_janela_exportar_servico__plano_vazio_ou_ilegivel_nao_quebra_sem() — - plano vazio ou ilegível não quebra: sem variáveis e lista []
function test_janela_exportar_servico__mostrar_o_pacote_sem_conexao_explica_o() — - mostrar o pacote sem conexão explica o que fazer
function test_janela_exportar_servico__o_setter_field_grava_a_chave_pedida() — - o setter field grava a chave pedida
local semente(n)
function test_janela_logs__o_historico_semeado_vira_texto_uma_linha_por() — - o histórico semeado vira texto, uma linha por evento, terminado em \\n
function test_janela_logs__sem_semente_a_janela_abre_vazia() — - sem semente a janela abre vazia
function test_janela_logs__semente_ilegivel_e_tratada_como_vazia() — - semente ilegível é tratada como vazia
function test_janela_logs__o_titulo_padrao_e_logs_e_o_que_o_pai_mandou_vale() — - o título padrão é Logs, e o que o pai mandou vale
function test_janela_novo_projeto__o_init_zera_mensagens_e_semeia_os_campos() — - o init zera mensagens e semeia os campos sem pisar no que veio
function test_janela_novo_projeto__a_falha_de_validacao_so_avisa() — - a falha de validação só avisa
function test_janela_novo_projeto__criar_sem_a_conexao_semeada_manda_reabrir() — - criar sem a conexão semeada manda reabrir a janela
function test_janela_novo_token_do_registry__o_init_abre_no_formulario_com() — - o init abre no formulário com escopo de leitura
function test_janela_novo_token_do_registry__os_escopos_oferecidos_sao_pull_e() — - os escopos oferecidos são pull e push
function test_janela_novo_token_do_registry__o_host_do_registry_vem_do_pai() — - o host do registry vem do pai, com um padrão local
function test_janela_novo_token_do_registry__criar_sem_conexao_e_a_falha_de() — - criar sem conexão e a falha de validação
local wizard(ctx_inicial)
function test_janela_novo_servico__abre_escolhendo_o_tipo_com_o_formulario() — - abre escolhendo o tipo, com o formulário zerado
function test_janela_novo_servico__pedida_para_importar_abre_direto_no_passo() — - pedida para importar, abre direto no passo de importação
function test_janela_novo_servico__o_passo_de_importacao_nasce_zerado() — - o passo de importação nasce zerado
function test_janela_novo_servico__catalogo_ilegivel_nao_derruba_a_janela() — - catálogo ilegível não derruba a janela
function test_janela_novo_servico__o_setter_field_existe_nesta_janela_digitar() — - o setter field existe nesta janela (digitar nos inputs escreve no ctx)

## rustploy-gui/vendor/iced_tiny_skia/src/

### engine.rs
struct Engine { text_pipeline, raster_pipeline, vector_pipeline }
impl Engine
  fn new() -> Self
  fn draw_quad(quad, background, transformation, pixels, clip_mask, clip_bounds)
  fn draw_text(text, transformation, pixels, clip_mask, clip_bounds)
  fn draw_primitive(primitive, transformation, pixels, clip_mask, clip_bounds)
  fn draw_image(image, _transformation, _pixels, _clip_mask, _clip_bounds)
  fn trim()
fn into_color(color) -> tiny_skia::Color
fn into_transform(transformation) -> tiny_skia::Transform
fn rounded_rectangle(bounds, border_radius) -> tiny_skia::Path
fn maybe_line_to(path, x, y)
fn arc_to(path, x_from, y_from, x_to, y_to, radius)
fn smoothstep(a, b, x) -> f32
fn rounded_box_sdf(to_center, size, radii) -> f32
fn adjust_clip_mask(clip_mask, bounds)

### geometry.rs
enum Geometry { Live, Cache }
struct Cache { text, images, primitives, clip_bounds }
impl Cached for Geometry: load, cache
struct Frame { clip_bounds, transform, stack, primitives, images, text }
impl Frame
  fn new(bounds) -> Self
impl geometry::frame::Backend for Frame: width, height, size, center, fill, fill_rectangle, stroke, stroke_rectangle, fill_text, stroke_text, push_transform, pop_transform, draft, paste, translate, rotate, scale, scale_nonuniform, into_geometry, draw_image, draw_svg
fn transform_rectangle(rectangle, transform) -> (Rectangle, Radians)
fn convert_path(path) -> Option<tiny_skia::Path>
fn into_paint(style) -> tiny_skia::Paint<'static>
fn into_fill_rule(rule) -> tiny_skia::FillRule
fn into_stroke(stroke) -> tiny_skia::Stroke

### layer.rs
type Stack = layer::Stack<Layer>
struct Layer { bounds, quads, primitives, images, text }
impl Layer
  fn draw_quad(quad, background, transformation)
  fn draw_paragraph(paragraph, position, color, clip_bounds, transformation)
  fn draw_editor(editor, position, color, clip_bounds, transformation)
  fn draw_text(text, position, color, clip_bounds, transformation)
  fn draw_text_raw(raw, transformation)
  fn draw_text_group(text, clip_bounds, transformation)
  fn draw_text_cache(text, clip_bounds, transformation)
  fn draw_image(image, transformation)
  fn draw_raster(image, bounds, clip_bounds, transformation)
  fn draw_svg(svg, bounds, clip_bounds, transformation)
  fn draw_primitive_group(primitives, clip_bounds, transformation)
  fn draw_primitive_cache(primitives, clip_bounds, transformation)
  fn damage(previous, current) -> Vec<Rectangle>
impl graphics::Layer for Layer: with_bounds, bounds, flush, resize, reset, start, end, merge
enum Item { Live, Group, Cached }
impl Item<T>
  fn transformation() -> Transformation
  fn clip_bounds() -> Rectangle
  fn as_slice() -> &[T]
impl Default for Layer

### lib.rs
struct Renderer { default_font, default_text_size, layers, engine } — A [`tiny-skia`] graphics renderer for [`iced`].
impl Renderer
  fn new(default_font, default_text_size) -> Self
  fn layers() -> &[Layer]
  fn draw(pixels, clip_mask, viewport, damage, background_color)
impl core::Renderer for Renderer: start_layer, end_layer, start_transformation, end_transformation, fill_quad, reset, allocate_image
impl core::text::Renderer for Renderer: default_font, default_size, fill_paragraph, fill_editor, fill_text
impl graphics::text::Renderer for Renderer: fill_raw
impl graphics::geometry::Renderer for Renderer: new_frame, draw_geometry
impl graphics::mesh::Renderer for Renderer: draw_mesh, draw_mesh_cache
impl core::image::Renderer for Renderer: load_image, measure_image, draw_image
impl core::svg::Renderer for Renderer: measure_svg, draw_svg
impl renderer::Headless for Renderer: new, name, screenshot
impl compositor::Default for Renderer

### primitive.rs
enum Primitive { Fill, Stroke }
impl Primitive
  fn visible_bounds() -> Rectangle — Returns the visible bounds of the [`Primitive`].

### raster.rs
struct Pipeline { cache }
impl Pipeline
  fn new() -> Self
  fn load(handle)
  fn dimensions(handle) -> Option<Size<u32>>
  fn draw(handle, filter_method, bounds, opacity, pixels, transform, clip_mask)
  fn trim_cache()
struct Cache { entries, hits }
impl Cache
  fn allocate(handle)
  fn trim()
struct Entry { width, height, pixels }

### settings.rs
struct Settings { default_font, default_text_size } — The settings of a [`Compositor`].
impl Default, From<graphics::Settings> for Settings

### text.rs
struct Pipeline { glyph_cache, cache }
impl Pipeline
  fn new() -> Self
  fn load_font(bytes)
  fn draw_paragraph(paragraph, position, color, pixels, clip_mask, transformation)
  fn draw_editor(editor, position, color, pixels, clip_mask, transformation)
  fn draw_cached(content, bounds, color, size, line_height, font, align_x, align_y, shaping, pixels, clip_mask, transformation)
  fn draw_raw(buffer, position, color, pixels, clip_mask, transformation)
  fn trim_cache()
fn draw(font_system, glyph_cache, buffer, position, color, pixels, clip_mask, transformation)
fn from_color(color) -> Color
struct GlyphCache { entries, recently_used, trim_count }
impl GlyphCache
  fn new() -> Self
  fn allocate(cache_key, color, font_system, swash) -> Option<(&[u8], cosmic_text::Placement)>
  fn trim()

### vector.rs
struct Pipeline { cache }
impl Pipeline
  fn new() -> Self
  fn viewport_dimensions(handle) -> Size<u32>
  fn draw(handle, color, bounds, opacity, pixels, transform, clip_mask)
  fn trim_cache()
struct Cache { trees, tree_hits, rasters, raster_hits, fontdb }
struct RasterKey { id, color, size }
impl Cache
  fn load(handle) -> Option<&usvg::Tree>
  fn viewport_dimensions(handle) -> Option<Size<u32>>
  fn draw(handle, color, size) -> Option<tiny_skia::PixmapRef<'_>>
  fn trim()
impl std::fmt::Debug for Cache

### window.rs

## rustploy-gui/vendor/iced_tiny_skia/src/window/

### compositor.rs
struct Compositor { context, settings }
struct Surface { window, clip_mask, layer_stack, background_color, max_age }
impl crate::graphics::Compositor for Compositor: with_backend, create_renderer, create_surface, configure_surface, information, present, screenshot
fn new(settings, display) -> Compositor
fn present(renderer, surface, viewport, background_color, on_pre_present) -> Result<(), compositor::SurfaceError>
fn screenshot(renderer, viewport, background_color) -> Vec<u8>
