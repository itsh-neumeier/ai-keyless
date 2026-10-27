<#
.SYNOPSIS
  AI-Keyless-Lab unter Windows (Docker Desktop) einrichten und bedienen.

.DESCRIPTION
  Reines PowerShell, keine Git Bash noetig. Bewusst nur ASCII-Text, damit
  Windows PowerShell 5.1 die Datei ohne BOM korrekt liest.

  Befehle:
    up      Secrets anlegen (falls fehlend), Image bauen, Stack starten
    token   Einmaliges Setup-Token ausgeben (erste Anmeldung)
    model   Vision-Modell in Ollama laden (Standard: granite3.2-vision)
    netlab  Synthetisches Netzwerk-Lab (host/bridge/macvlan) in der Docker-VM
    status  Container-Status anzeigen
    logs    Logs der API verfolgen
    down    Stack stoppen (Daten bleiben in den Volumes erhalten)

.EXAMPLE
  cd D:\GitHub\ai-keyless
  powershell -ExecutionPolicy Bypass -File scripts\lab.ps1 up
#>
param(
    [Parameter(Position = 0)]
    [ValidateSet('up', 'token', 'model', 'netlab', 'status', 'logs', 'down')]
    [string]$Command = 'up',
    [string]$Model = 'granite3.2-vision',
    [switch]$NoBuild,
    [switch]$NoGpu
)

$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$composeDir = 'deploy\compose'
$secretsDir = Join-Path $composeDir 'secrets'
$envFile = Join-Path $composeDir '.env'

function Compose {
    $files = @('-f', "$composeDir\compose.yaml")
    if (-not $NoGpu) { $files += @('-f', "$composeDir\compose.lab.yaml") }
    & docker compose @files --env-file $envFile @args
    if ($LASTEXITCODE -ne 0) { throw "docker compose $args fehlgeschlagen (Exit $LASTEXITCODE)" }
}

function Initialize-Env {
    if (-not (Test-Path $envFile)) {
        Copy-Item "$composeDir\.env.example" $envFile
        Write-Host "Angelegt: $envFile"
    }
}

function Initialize-Secrets {
    if (Test-Path (Join-Path $secretsDir 'db_password')) {
        Write-Host "Secrets existieren bereits in $secretsDir - nichts ueberschrieben."
        return
    }
    New-Item -ItemType Directory -Force $secretsDir | Out-Null
    $bytes = New-Object byte[] 32
    [Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    $pw = ([Convert]::ToBase64String($bytes) -replace '[^A-Za-z0-9]', '')
    $root = (Resolve-Path $secretsDir).Path
    # Ohne BOM und ohne Zeilenende schreiben (PostgreSQL liest die Datei roh).
    [IO.File]::WriteAllText((Join-Path $root 'db_password'), $pw)
    [IO.File]::WriteAllText((Join-Path $root 'database_url'), "postgres://akl:$pw@db:5432/akl")
    Write-Host "Secrets erzeugt in $secretsDir (nicht in Git)."
}

function Assert-Docker {
    & docker info *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Docker laeuft nicht - Docker Desktop starten.' }
}

Assert-Docker
switch ($Command) {
    'up' {
        Initialize-Env
        Initialize-Secrets
        if (-not $NoBuild) {
            & docker build -f deploy/docker/Dockerfile -t ai-keyless:dev .
            if ($LASTEXITCODE -ne 0) { throw 'Image-Build fehlgeschlagen.' }
        }
        Compose up -d
        Write-Host ''
        Write-Host 'Gestartet. Naechste Schritte:'
        Write-Host '  scripts\lab.ps1 token   -> Setup-Token fuer http://127.0.0.1:8090'
        if (-not $NoGpu) { Write-Host "  scripts\lab.ps1 model   -> $Model in Ollama laden" }
    }
    'token' { Compose exec api ai-keyless-api setup-token }
    'model' {
        if ($NoGpu) { throw 'Ohne Lab-Overlay gibt es keinen Ollama-Container.' }
        Compose exec ollama ollama pull $Model
    }
    'netlab' {
        # scripts/netlab.sh braucht root, ip und nsenter im Netz-/PID-Namespace
        # des Docker-Daemons; unter Docker Desktop liefert das ein Hilfscontainer.
        # Alles bleibt in der Docker-VM (eigene Bridge), kein Verkehr ins LAN.
        $scripts = (Resolve-Path 'scripts').Path
        & docker run --rm --privileged --network host --pid host `
            -v /var/run/docker.sock:/var/run/docker.sock -v "${scripts}:/scripts:ro" `
            docker:27.5.1-cli sh -c 'apk add -q bash iproute2 util-linux && bash /scripts/netlab.sh'
        if ($LASTEXITCODE -ne 0) { throw 'Netzwerk-Lab fehlgeschlagen.' }
    }
    'status' { Compose ps }
    'logs' { Compose logs -f api }
    'down' { Compose down }
}
