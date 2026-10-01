# vendor/

## iced_tiny_skia (0.14.0, patch de 1 trecho)

Cópia do `iced_tiny_skia` 0.14.0 com UM ajuste em `src/engine.rs` (arm
`Text::Editor`, marcado `RUSTPLOY`): o recorte do `text_editor` passa a ser
sempre aplicado. No upstream ele é pulado quando `editor.bounds` cabe no
recorte, mas o editor desenha linhas parciais além dessa caixa — no renderer de
software, a linha cortada na borda de cima/baixo de um `<textarea>` saía inteira
e sobreposta à vizinha (visto na janela de logs). A 0.14.1 tem o mesmo defeito.

Ativado por `[patch.crates-io]` no `Cargo.toml` da raiz. O app cai no tiny-skia
quando o wgpu/GL não sobe (ex.: Intel Ivy Bridge, `WGPU_BACKEND=gl`).

O upstream JÁ corrigiu (commit `23170119b`, 2026-01-28, "Always clip
`Text::Editor` in `tiny_skia`"), mas só na `master` (pós-0.14.x). Remover esta
pasta e o `[patch.crates-io]` quando o glacier-ui migrar para um iced que
contenha esse commit (0.15+).
