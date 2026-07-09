--!strict
-- fmt.luau — fachada: reexporta os builders de views/scripts/fmt_*.luau sob um
-- único módulo, para os handlers seguirem fazendo `local fmt = require("fmt")`
-- + `fmt.foo(...)` sem precisar saber em qual submódulo cada builder mora.
-- Reimplementação Luau de `src/app/net/view.rs` (Rust pré-migração).

local Time = require("fmt/time")
local Util = require("fmt/util")
local Dashboard = require("fmt/dashboard")
local ServiceDetail = require("fmt/service_detail")
local Git = require("fmt/git")

local M = {}

-- Tempo / números (fmt/time.luau)
M.time_hms = Time.time_hms
M.date_dm_hm = Time.date_dm_hm
M.date_dm_hms = Time.date_dm_hms
M.fmt_secs = Time.fmt_secs
M.fmt_uptime = Time.fmt_uptime
M.fmt_bytes = Time.fmt_bytes
M.progress_bar = Time.progress_bar

-- Busca / codificação / estado (fmt/util.luau)
M.matches = Util.matches
M.enc_array = Util.enc_array
M.status_label_color = Util.status_label_color
M.pair_list = Util.pair_list

-- Dashboard: deployments/projects/services/docker/ingress/monitoring/engine
-- (fmt/dashboard.luau)
M.deployments = Dashboard.deployments
M.services = Dashboard.services
M.service_rows = Dashboard.service_rows
M.projects = Dashboard.projects
M.project_rows = Dashboard.project_rows
M.docker_rows = Dashboard.docker_rows
M.docker_images = Dashboard.docker_images
M.docker_volumes = Dashboard.docker_volumes
M.docker_networks = Dashboard.docker_networks
M.ingress = Dashboard.ingress
M.monitoring = Dashboard.monitoring
M.eng_active = Dashboard.eng_active
M.eng_recent = Dashboard.eng_recent

-- Service detail: source/healthcheck/env/logs/deployments/domains/connection
-- (fmt_service_detail.luau)
M.source_summary3 = ServiceDetail.source_summary3
M.healthcheck_summary = ServiceDetail.healthcheck_summary
M.env_json_with_comments = ServiceDetail.env_json_with_comments
M.env_dotenv_with_comments = ServiceDetail.env_dotenv_with_comments
M.logs_json = ServiceDetail.logs_json
M.build_logs_json = ServiceDetail.build_logs_json
M.join_log_lines = ServiceDetail.join_log_lines
M.deployments_detail = ServiceDetail.deployments_detail
M.domains_json = ServiceDetail.domains_json
M.safe_name = ServiceDetail.safe_name
M.internal_url = ServiceDetail.internal_url
M.external_url = ServiceDetail.external_url
M.containers = ServiceDetail.containers

-- Git providers/repos/branches (fmt/git.luau)
M.git_providers = Git.git_providers
M.git_repos = Git.git_repos
M.git_branches = Git.git_branches

return M
