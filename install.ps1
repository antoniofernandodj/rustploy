# Instalador do app desktop do Rustploy para Windows, a partir da última release do GitHub.
#
#   irm https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.ps1 | iex
#
# Instala por usuário (não precisa de administrador) em %LOCALAPPDATA%\Rustploy e cria um
# atalho no Menu Iniciar. Rodar de novo atualiza.
#
# Variáveis de ambiente (opcionais, defina antes do comando):
#   $env:RUSTPLOY_VERSION = 'v0.1.1'   uma release específica (padrão: a última)
#   $env:RUSTPLOY_DRY_RUN = '1'        só mostra o que baixaria e instalaria
#   $env:RUSTPLOY_NO_VERIFY = '1'      instala SEM conferir a assinatura minisign (não recomendado)
#
# Linux/Debian/Ubuntu: veja install.sh.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # o Invoke-WebRequest fica muito mais rápido sem a barra

$Repo = if ($env:RUSTPLOY_REPO) { $env:RUSTPLOY_REPO } else { 'antoniofernandodj/rustploy' }
$Api = if ($env:RUSTPLOY_API_URL) { $env:RUSTPLOY_API_URL } else { "https://api.github.com/repos/$Repo/releases" }
$Docs = 'https://rustploy-docs.antoniofernandodj.workers.dev/instalacao'
$Dry = $env:RUSTPLOY_DRY_RUN -eq '1'
$NoVerify = $env:RUSTPLOY_NO_VERIFY -eq '1'

# Chaves públicas minisign dos pacotes, uma por máquina que assina releases (as mesmas de
# keys/, dos updaters e do install.sh); qualquer uma basta. RUSTPLOY_TRUSTED_KEY troca a lista, só para testes.
$TrustedKeys = if ($env:RUSTPLOY_TRUSTED_KEY) { $env:RUSTPLOY_TRUSTED_KEY -split '\s+' } else { @(
    'RWSWAQvUAwKjxh3dDwqvzjm9XPx/tEqDqrctDWMlwA22TgSzy/0V2kP/',
    'RWS1siYVw0N6jJgNiQe25EyQakKI/4c7GBSUZZmydt0KUEXaUf8jHL1j'
) }

# O Windows não traz como conferir ed25519/BLAKE2b, então baixamos o minisign oficial. O SHA-256
# fixo aqui é o do zip publicado pelo autor (conferido com a chave dele,
# RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3): vem junto com as chaves acima.
$MinisignUrl = 'https://github.com/jedisct1/minisign/releases/download/0.12/minisign-0.12-win64.zip'
$MinisignSha256 = '37b600344e20c19314b2e82813db2bfdcc408b77b876f7727889dbd46d539479'

function Say($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Warn($msg) { Write-Host "aviso: $msg" -ForegroundColor Yellow }
function Fail($msg) { Write-Host "erro: $msg" -ForegroundColor Red; throw $msg }

if (-not [Environment]::Is64BitOperatingSystem) { Fail 'o app desktop só tem versão para Windows 64 bits.' }
if ($PSVersionTable.PSVersion.Major -lt 5) { Fail 'precisa do PowerShell 5 ou mais novo.' }
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Url = if ($env:RUSTPLOY_VERSION) { "$Api/tags/$($env:RUSTPLOY_VERSION)" } else { "$Api/latest" }
Say "Procurando a release $(if ($env:RUSTPLOY_VERSION) { $env:RUSTPLOY_VERSION } else { 'mais recente' }) de $Repo..."

try {
    $Release = Invoke-RestMethod -Uri $Url -Headers @{ 'User-Agent' = 'rustploy-installer'; 'Accept' = 'application/vnd.github+json' }
} catch {
    $Code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
    if ($Code -eq 404 -and $env:RUSTPLOY_VERSION) {
        Fail "a release $($env:RUSTPLOY_VERSION) não existe em github.com/$Repo. Veja as disponíveis em https://github.com/$Repo/releases"
    }
    if ($Code -eq 404) {
        Fail "nenhuma release publicada em github.com/$Repo. Enquanto não houver, instale pelo crates.io (cargo install rustploy-gui); veja $Docs/instalar-clientes/"
    }
    if ($Code -eq 403 -or $Code -eq 429) {
        Fail "o GitHub limitou as consultas à API (HTTP $Code). Espere alguns minutos ou defina `$env:RUSTPLOY_VERSION = 'vX.Y.Z'."
    }
    Fail "não consegui consultar $Url ($($_.Exception.Message))"
}

$Zip = $Release.assets | Where-Object { $_.name -like 'rustploy-gui-windows-*.zip' } | Select-Object -First 1
if (-not $Zip) { Fail "a release $($Release.tag_name) não tem o zip do Windows (rustploy-gui-windows-*.zip)." }
$Sums = $Release.assets | Where-Object { $_.name -eq 'checksums.sha256' } | Select-Object -First 1
$Sig = $Release.assets | Where-Object { $_.name -eq "$($Zip.name).minisig" } | Select-Object -First 1
if (-not $Sig -and -not $NoVerify) {
    Fail "a release $($Release.tag_name) não tem assinatura ($($Zip.name).minisig). Não instalei nada. Se confia nela mesmo assim: `$env:RUSTPLOY_NO_VERIFY = '1'"
}

Say "Release $($Release.tag_name): $($Zip.name)"
$Dest = Join-Path $env:LOCALAPPDATA 'Rustploy'
if ($Dry) {
    Say "--dry-run: baixaria $($Zip.browser_download_url)"
    Say "--dry-run: e instalaria em $Dest (com atalho no Menu Iniciar)"
    return
}

$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("rustploy-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
    $ZipPath = Join-Path $Tmp $Zip.name
    Say "Baixando $($Zip.name)..."
    Invoke-WebRequest -Uri $Zip.browser_download_url -OutFile $ZipPath -UseBasicParsing

    if ($Sums) {
        $SumsPath = Join-Path $Tmp 'checksums.sha256'
        Invoke-WebRequest -Uri $Sums.browser_download_url -OutFile $SumsPath -UseBasicParsing
        $Line = Get-Content $SumsPath | Where-Object { $_ -match [Regex]::Escape($Zip.name) + '$' } | Select-Object -First 1
        if (-not $Line) { Fail "o checksums.sha256 da release não lista $($Zip.name)." }
        $Want = ($Line -split '\s+')[0].ToLower()
        $Got = (Get-FileHash -Algorithm SHA256 -Path $ZipPath).Hash.ToLower()
        if ($Want -ne $Got) { Fail "SHA-256 de $($Zip.name) não confere (esperado $Want, obtido $Got). Não instalei nada." }
        Say 'SHA-256 conferido.'
    } else {
        Warn 'a release não tem checksums.sha256; não consegui conferir o SHA-256.'
    }

    # Assinatura: o SHA-256 vem do mesmo lugar do zip e não prova autoria; a assinatura
    # minisign (chave só do dono, fora do GitHub) prova. Sem ela, não instala.
    if ($NoVerify) {
        Warn 'RUSTPLOY_NO_VERIFY: instalando SEM conferir a assinatura do zip.'
    } else {
        $Minisign = Get-Command minisign -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
        if (-not $Minisign) {
            Say 'Baixando o minisign (para conferir a assinatura)...'
            $MinisignZip = Join-Path $Tmp 'minisign.zip'
            Invoke-WebRequest -Uri $MinisignUrl -OutFile $MinisignZip -UseBasicParsing
            $Got = (Get-FileHash -Algorithm SHA256 -Path $MinisignZip).Hash.ToLower()
            if ($Got -ne $MinisignSha256) { Fail "SHA-256 do minisign baixado não confere (esperado $MinisignSha256, obtido $Got). Não instalei nada." }
            $MinisignDir = Join-Path $Tmp 'minisign'
            Expand-Archive -Path $MinisignZip -DestinationPath $MinisignDir -Force
            $MinisignArch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
            $Minisign = Join-Path $MinisignDir "minisign-win64\$MinisignArch\minisign.exe"
        }
        $SigPath = "$ZipPath.minisig"
        Invoke-WebRequest -Uri $Sig.browser_download_url -OutFile $SigPath -UseBasicParsing
        $Verified = $null
        foreach ($Key in $TrustedKeys) {
            # 'Continue' só aqui: no PowerShell 5.1, com 'Stop', o stderr do minisign (chave que não é
            # a que assinou) viraria exceção antes de tentar a próxima chave.
            $Output = & { $ErrorActionPreference = 'Continue'; & $Minisign -V -P $Key -x $SigPath -m $ZipPath 2>&1 | ForEach-Object { "$_" } } | Out-String
            if ($LASTEXITCODE -eq 0) { $Verified = $Output; break }
        }
        if (-not $Verified) { Fail "a assinatura de $($Zip.name) NÃO confere com nenhuma chave do Rustploy. Não instalei nada. $Output" }
        # O comentário assinado diz qual pacote/versão é: impede passar um zip assinado de
        # outra versão (um downgrade) ou de outro pacote no lugar deste.
        $Claim = ($Verified -split "`r?`n" | Where-Object { $_ -like 'Trusted comment: *' } | Select-Object -First 1) -replace '^Trusted comment: ', ''
        $Version = $Zip.name -replace '^rustploy-gui-windows-v', '' -replace '\.zip$', ''
        $WantClaim = "package=rustploy-gui-windows version=$Version arch=amd64"
        if ($Claim -ne $WantClaim) { Fail "a assinatura é de outro pacote (assinado: '$Claim'; baixado: '$WantClaim'). Não instalei nada." }
        Say "Assinatura conferida ($Claim)."
    }

    Get-Process -Name 'rustploy-gui' -ErrorAction SilentlyContinue | ForEach-Object {
        Warn 'o Rustploy está aberto; fechando para atualizar.'
        $_ | Stop-Process -Force
    }

    Say "Instalando em $Dest..."
    Expand-Archive -Path $ZipPath -DestinationPath $Tmp -Force
    $Src = Get-ChildItem -Path $Tmp -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'rustploy-gui.exe') } | Select-Object -First 1
    if (-not $Src) { Fail 'o zip não tem rustploy-gui.exe.' }
    if (Test-Path $Dest) { Remove-Item -Recurse -Force $Dest }
    New-Item -ItemType Directory -Path $Dest | Out-Null
    Copy-Item -Path (Join-Path $Src.FullName '*') -Destination $Dest -Recurse -Force

    $Menu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
    $Shell = New-Object -ComObject WScript.Shell
    $Link = $Shell.CreateShortcut((Join-Path $Menu 'Rustploy.lnk'))
    $Link.TargetPath = Join-Path $Dest 'rustploy-gui.exe'
    $Link.WorkingDirectory = $Dest
    $Link.Description = 'Rustploy: cliente desktop do PaaS'
    $Link.Save()
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}

Say "Pronto. Abra o Rustploy pelo Menu Iniciar, ou execute $Dest\rustploy-gui.exe"
Say "Na tela de login, cole o token de conexão do seu servidor (veja $Docs/primeiro-acesso/)."
Warn 'o executável ainda não tem assinatura que o SmartScreen reconheça. Se aparecer o aviso: Mais informações > Executar assim mesmo.'
