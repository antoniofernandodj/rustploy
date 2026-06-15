# Conceitos: TLS, Certificados, CSR, CN, SANs, ACME e rcgen

Este documento explica do zero os conceitos por trás do HTTPS — sem assumir
conhecimento prévio. Serve como referência para entender o que acontece dentro
do `TlsManager` do Rustploy e o bug que corrigimos em 2026-06-15.

---

## O problema que o HTTPS resolve

Quando você acessa um site em HTTP puro, os dados viajam "em texto claro" pela
internet. Qualquer roteador no caminho entre você e o servidor pode ler (ou
alterar) o conteúdo. HTTPS adiciona uma camada de criptografia — o **TLS** —
que garante:

1. **Confidencialidade**: ninguém no meio do caminho lê o conteúdo.
2. **Integridade**: ninguém consegue alterar os dados sem ser detectado.
3. **Autenticidade**: você tem como verificar que está falando com o servidor
   certo, e não com um impostor.

O terceiro ponto é o mais interessante — e é aí que entram os **certificados**.

---

## O que é um certificado TLS

Um certificado é um documento digital que diz:

> "O servidor que responde pelo domínio `chiquitos.tech` tem esta chave
> pública, e eu (Let's Encrypt) confirmo isso."

Ele contém:
- **Para qual(is) domínio(s) vale** (ex: `chiquitos.tech`).
- **A chave pública** do servidor.
- **Quem assinou** (a Autoridade Certificadora — CA).
- **Validade** (Let's Encrypt emite por 90 dias).

O navegador já vem com uma lista de CAs confiáveis. Quando o servidor apresenta
um certificado assinado por uma delas, o navegador aceita sem perguntar.
Quando não há certificado, ou ele é inválido, aparece o erro
`ERR_SSL_PROTOCOL_ERROR` — exatamente o que estávamos vendo.

---

## O que é uma CA (Certificate Authority)

Uma CA é uma entidade em quem os navegadores confiam para atestar a identidade
de domínios. As mais conhecidas são Let's Encrypt (gratuita e automática),
DigiCert e Sectigo (pagas).

Para emitir um certificado para `chiquitos.tech`, a CA precisa ter certeza de
que quem está pedindo o cert realmente controla aquele domínio. Esse processo
de verificação é chamado de **challenge** (desafio).

---

## O que é ACME

ACME (Automatic Certificate Management Environment) é o protocolo que o
Let's Encrypt criou para automatizar a emissão de certificados. Em vez de
mandar e-mail e esperar aprovação manual, o servidor prova que controla o
domínio automaticamente.

O desafio que o Rustploy usa é o **HTTP-01**:

1. O Rustploy pede um cert ao Let's Encrypt para `chiquitos.tech`.
2. O LE responde: "coloque este token em
   `http://chiquitos.tech/.well-known/acme-challenge/<token>`".
3. O Rustploy armazena o token e responde às requisições nessa URL.
4. O LE acessa a URL e confirma que o token está lá.
5. LE conclui: "quem pediu o cert claramente controla o domínio" → emite.

Isso só funciona se a porta 80 estiver acessível publicamente e o DNS do
domínio estiver apontando para o servidor.

---

## O que é um CSR (Certificate Signing Request)

Depois que o desafio é validado, o servidor ainda não tem o certificado.
Ele precisa enviar ao LE um **CSR** — uma "solicitação de assinatura de
certificado".

O CSR é um arquivo que contém:
- **A chave pública** que o servidor gerou localmente.
- **Para qual domínio** o cert deve ser emitido.
- Uma **assinatura digital** que prova que quem está pedindo tem a
  chave privada correspondente à chave pública.

O servidor gera o par de chaves (pública + privada) localmente e **nunca
envia a chave privada para ninguém**. O LE recebe só o CSR (com a chave
pública), assina, e devolve o certificado. Com o certificado em mãos, o
servidor apresenta as duas coisas ao navegador: o cert (público) e a chave
privada (só ele tem).

Resumindo o fluxo:

```
Servidor gera:  chave_privada + chave_pública
Servidor cria:  CSR  =  { chave_pública, domínio, assinatura }
Servidor envia: CSR → Let's Encrypt
LE assina e devolve: certificado (contém a chave pública + assinatura do LE)

Na conexão HTTPS:
  Servidor apresenta: certificado + prova de que tem a chave_privada
  Navegador verifica: a assinatura do LE é válida? o domínio bate? → confia
```

---

## O que é CN (Common Name)

Dentro do CSR existe um campo chamado **Distinguished Name (DN)**, que
descreve "quem está pedindo". O DN tem sub-campos:

| Campo | Abreviação | Exemplo |
|---|---|---|
| Common Name | CN | `chiquitos.tech` |
| Organization | O | `Minha Empresa Ltda` |
| Country | C | `BR` |

Historicamente, o CN era o campo principal para indicar o domínio.
Hoje em dia, o que importa de verdade são os **SANs** (veja abaixo),
mas o CN ainda existe nos CSRs.

---

## O que são SANs (Subject Alternative Names)

SANs são a forma moderna de indicar para quais domínios um certificado vale.
Um único cert pode cobrir vários domínios:

```
SAN: chiquitos.tech
SAN: www.chiquitos.tech
SAN: api.chiquitos.tech
```

Navegadores modernos verificam os SANs, não o CN. O CN virou legado. Let's
Encrypt olha principalmente os SANs para decidir o que assinar.

---

## O que é rcgen

`rcgen` é uma biblioteca Rust para gerar chaves e CSRs. O Rustploy usa ela
para criar o par de chaves e montar o CSR antes de enviar ao LE.

O código é:

```rust
let key_pair = KeyPair::generate()?;         // gera chave privada + pública
let params   = CertificateParams::new(vec!["chiquitos.tech"])?; // SAN
let csr      = params.serialize_request(&key_pair)?;  // monta o CSR
```

---

## O bug que corrigimos

`CertificateParams::new()` do rcgen cria os parâmetros com um **CN padrão**:

```
CN = "rcgen self signed cert"
```

Esse é um valor que a biblioteca usa internamente para gerar certificados
auto-assinados (para testes). Quando usamos `CertificateParams` para gerar
um **CSR de produção**, o CN padrão vai junto.

O Let's Encrypt recebeu o CSR, viu o CN `"rcgen self signed cert"`, tentou
interpretá-lo como nome de domínio (o que historicamente o CN representa),
e recusou com:

```
Cannot issue for "rcgen self signed cert":
Domain name contains an invalid character
(urn:ietf:params:acme:error:rejectedIdentifier)
```

O espaço no meio da string é o "invalid character" — não é um hostname válido.

### A correção

Substituímos o DN padrão por um DN vazio:

```rust
let mut params = CertificateParams::new(vec!["chiquitos.tech"])?;
params.distinguished_name = DistinguishedName::new(); // sem CN
let csr = params.serialize_request(&key_pair)?;
```

Sem CN, o LE olha apenas os SANs — que têm o domínio correto — e emite o
certificado normalmente. Isso é o comportamento esperado no ACME moderno:
o CN é ignorado, só os SANs importam.

---

## Resumo visual

```
Você no navegador
        │
        │  "quero acessar https://chiquitos.tech"
        ▼
    Servidor (Rustploy)
        │
        │  "aqui está meu certificado"
        │  (assinado pelo Let's Encrypt, válido para chiquitos.tech)
        ▼
    Navegador verifica:
        ├─ A assinatura é do Let's Encrypt? ✓ (CA confiável)
        ├─ O cert é para chiquitos.tech?    ✓ (SAN bate com o domínio)
        └─ Ainda está dentro da validade?   ✓ (emitido há menos de 90 dias)
        → Conexão estabelecida, cadeado verde 🔒

Como o cert foi gerado (bastidores):
        Rustploy → "quero cert para chiquitos.tech" → Let's Encrypt
        LE       → "prove que controla o domínio"   → challenge HTTP-01
        Rustploy → serve token em /.well-known/...  → LE valida
        Rustploy → envia CSR (chave pública + SAN)  → LE
        LE       → devolve certificado assinado      → Rustploy salva em disco
```
