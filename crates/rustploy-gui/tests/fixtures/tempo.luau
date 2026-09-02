--!strict
-- Fixture do teste `fmt_time.rs`: exercita o `fmt/time.luau` de verdade,
-- através do motor, e deixa cada resultado numa chave de contexto.
--
-- O `require("fmt/time")` resolve por `GLACIER_LUAU_PATH`, que o teste aponta
-- para `views/scripts` — este arquivo mora em `tests/fixtures/`, fora da
-- árvore da aplicação, justamente para não virar script do app.
local Time = require("fmt/time")

function init(): ()
    -- O MESMO instante escrito de duas formas. Independe do fuso da máquina:
    -- as duas têm de renderizar igual, e é isso que prova a conversão.
    ctx.hms_z = Time.time_hms("2026-07-06T12:34:56Z")
    ctx.hms_off = Time.time_hms("2026-07-06T09:34:56-03:00")
    -- Fração de segundo entra e não muda o resultado.
    ctx.hms_frac = Time.time_hms("2026-07-06T12:34:56.789Z")

    ctx.dm_hm = Time.date_dm_hm("2026-07-06T12:34:56Z")
    ctx.dm_hms = Time.date_dm_hms("2026-07-06T12:34:56Z")

    -- Entrada ausente ou malformada continua virando "" — é o caso normal de
    -- um `finished_at` que o daemon ainda não preencheu.
    ctx.vazio_nil = Time.time_hms(nil)
    ctx.vazio_lixo = Time.time_hms("nao é data")

    -- Duração entre dois instantes com fuso: absoluta, não depende do fuso.
    ctx.dur = Time.fmt_duration({
        started_at = "2026-07-06T12:00:00Z",
        finished_at = "2026-07-06T12:01:30Z",
    })
    -- Sem `finished_at` conta até agora; só precisa não explodir.
    ctx.dur_aberta = Time.fmt_duration({ started_at = "2026-07-06T12:00:00Z" })
    -- `started_at` inválido cai no default.
    ctx.dur_invalida = Time.fmt_duration({ started_at = "" })
end
