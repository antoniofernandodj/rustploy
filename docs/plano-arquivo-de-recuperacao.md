# Plano: arquivo de recuperação (`.rpkit`)

> **Status (2026-11-18): IMPLEMENTADO, não publicado.** Daemon, GUI e webui prontos e testados. O
> `glacier-ui` 0.168.0 (bytes no `write_file`/`save_download`, `read_file` e `base64`) está commitado e
> enviado, mas **não publicado no crates.io**: a GUI depende dele (`Cargo.toml` já pede 0.168.0) e só
> compila depois do `cargo publish`. **Não visto rodando:** as telas numa janela real (GUI) e no navegador
> com um daemon de verdade; uma recuperação completa a partir do arquivo.

## Por quê

Num desastre, a tela Recuperar pedia quatro coisas, de quatro lugares: o endereço do repositório da
infra, a branch, um token de acesso a esse repositório e a chave de recuperação privada. A chave já
saía num arquivo; o resto a pessoa tinha de lembrar ou achar, justo no pior momento.

## A ideia, e por que ela só funciona na hora de gerar a chave

A chave privada só existe **uma vez**: o servidor a gera, grava só a pública e a devolve ao cliente na
resposta de `RecoveryKeyGenerate`. Então o arquivo que reúne tudo tem de nascer nesse instante. O
servidor tem as outras peças à mão: o token da conta Git está guardado nele. Por isso:

- **gerar a chave pede, junto, o repositório (obrigatório), a branch (opcional) e a conta Git
  (obrigatória para `https://`)**, escolhida num select das contas cadastradas. O formulário começa com
  o que o Git Sync já usa;
- **o daemon monta o arquivo** e o devolve em base64; o token nunca precisa ser mandado ao cliente por
  outra via;
- **tudo é conferido antes de gerar**: repositório vazio, SSH, conta faltando ou inexistente, ou um
  repositório de outro host que não o da conta são recusados **sem trocar a chave**;
- o token só entra se o repositório for do host da conta (`usable_token_for_url`): quem escolhe a URL
  não consegue levar, no arquivo, o token do GitHub para outro servidor;
- conta **OAuth** gera com aviso: o token pode expirar antes do desastre. O recomendado é um token
  pessoal (PAT) só de leitura do repositório da infra.

Na tela Recuperar, **"Carregar arquivo de recuperação…"** lê o arquivo e preenche os quatro campos, que
continuam editáveis (um token vencido se troca à mão).

## O formato

Binário, inteiros little-endian, sem lib nova (só `std` no Rust, `bit32`/`string.byte` no Luau,
`DataView` no JS):

```text
offset  bytes  conteúdo
0       8      assinatura: 89 'R' 'P' 'K' 'I' 'T' 0D 0A
8       2      versão (u16 = 1)
10      2      quantidade de campos (u16)
12      …      campos: etiqueta (u16) | tamanho (u32) | dados (UTF-8)
fim-4   4      CRC-32 (IEEE) de tudo o que vem antes (u32)
```

Etiquetas: 1 repositório, 2 branch, 3 token, 4 chave privada, 5 chave pública, 6 servidor de origem,
7 data de geração. Etiqueta desconhecida é pulada (uma versão futura acrescenta campos sem quebrar os
leitores de hoje).

- **Não se corrompe em silêncio.** Qualquer byte mudado, faltando ou sobrando é pego pelo CRC; a
  assinatura denuncia cópia em modo texto (o `0x89` perde o bit alto) e conversão de fim de linha (o
  `0D 0A`), cada um com mensagem própria. Os três leitores são testados trocando cada byte do arquivo.
- **Não é cifrado.** Quem tem o arquivo tem a chave e o token: as telas dizem para guardá-lo num
  cofre. Proteger com senha (`age -p`) fica como próximo passo, se for o caso.

Os três leitores — `features/data_backup/recovery_kit.rs` (daemon, também o escritor),
`views/scripts/recovery_kit.luau` (GUI) e `webui/recovery_kit.js` — conferem o **mesmo vetor de bytes**
(`vetor_compartilhado_com_os_clientes`). Mudou o formato: mude os três e o vetor.

## O que mudou no glacier-ui (0.168.0)

A GUI precisava gravar e ler bytes, e o glacier 0.167 não deixava: `write_file`, `append_file` e
`save_download` recebiam `String` do Rust (recusam o que não é UTF-8), e a leitura era só pelo
`fetch("file://…")` (texto, ou base64 sem decodificador no Luau). Na 0.168:

- os três gravadores levam a string Luau como **bytes**;
- `read_file(caminho)` lê em bytes crus, sem suspender;
- `base64.encode`/`base64.decode` (a resposta JSON traz o arquivo em base64).

## Protocolo

`RecoveryKeyGenerate { replace, infra_remote, infra_branch?, provider_id? }` →
`RecoveryKeyGenerated { public_key, private_key, file_base64, suggested_filename, warning? }`. O nome
sugerido termina em `.rpkit`. Um app antigo (que não manda `infra_remote`) recebe `InvalidInput` dizendo
para atualizar; a chave não é trocada. `RecoveryKeyGenerate` continua proibido ao agente MCP.

## Verificação

Feito: testes do formato no daemon (ida e volta, CRC de referência, cada byte trocado, truncado, sobra,
modo texto, fim de linha), do handler (pedido inválido não troca a chave; o token só entra para o host
da conta; OAuth avisa; remoto local sem conta), suíte inteira do daemon (699 verdes), leitor Luau e
telas da GUI (480 verdes, contra o glacier local via `--config patch`), leitor JS em Node contra o vetor,
e um teste headless no Chrome (gerar manda os campos e baixa o binário; carregar preenche os campos;
um byte trocado é recusado).
