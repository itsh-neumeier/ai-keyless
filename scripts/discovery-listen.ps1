<#
.SYNOPSIS
  TP-002: Discovery-Datagramme im LAN passiv empfangen und protokollieren.

.DESCRIPTION
  Bindet UDP auf dem Windows-Host (nicht in Docker: Docker Desktop sieht
  LAN-Broadcasts nicht, siehe docs/operations/NETWORKING.md) und schreibt
  jedes empfangene Datagramm als Zeile in eine lokale Datei:
  Zeit, Absender, Zieladresse (Broadcast/Unicast/Multicast), Laenge, Hex,
  Kopf (Version/Kommando/Laenge) laut Hypothese DISC-H04.

  Sendet NICHTS (keine Antworten, keine Suchanfragen, kein Scan).
  Die Ausgabe kann MAC-Adressen u. a. enthalten -> bleibt lokal, nur
  redigiert ins Repository (docs/compatibility/evidence/).
  Bewusst nur ASCII-Text (Windows PowerShell 5.1).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\discovery-listen.ps1 -Minutes 10
#>
param(
    [int]$Port = 10001,
    [int]$Minutes = 10,
    [string]$OutFile = (Join-Path $env:TEMP ("akl-tp002-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')),
    # Optional: Multicast-Gruppe beitreten (nur wenn in TP-002 freigegeben und belegt).
    [string]$Multicast = ''
)

$ErrorActionPreference = 'Stop'

$client = New-Object System.Net.Sockets.UdpClient
$client.Client.SetSocketOption(
    [System.Net.Sockets.SocketOptionLevel]::Socket,
    [System.Net.Sockets.SocketOptionName]::ReuseAddress, $true)
$client.Client.Bind((New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, $Port)))
if ($Multicast) {
    $client.JoinMulticastGroup([System.Net.IPAddress]::Parse($Multicast))
}
$client.Client.ReceiveTimeout = 1000
# Zieladresse je Datagramm mitliefern (Broadcast, Unicast oder Multicast).
$client.Client.SetSocketOption(
    [System.Net.Sockets.SocketOptionLevel]::IP,
    [System.Net.Sockets.SocketOptionName]::PacketInformation, $true)
$buffer = New-Object byte[] 2048

$deadline = (Get-Date).AddMinutes($Minutes)
$count = 0
"# TP-002 passiver Empfang UDP $Port, Start $(Get-Date -Format o), Multicast '$Multicast'" | Out-File -Encoding ascii $OutFile
Write-Host "Lausche auf UDP $Port bis $($deadline.ToString('HH:mm:ss')) -> $OutFile"

try {
    while ((Get-Date) -lt $deadline) {
        [System.Net.EndPoint]$remote = New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, 0)
        $flags = [System.Net.Sockets.SocketFlags]::None
        $info = New-Object System.Net.Sockets.IPPacketInformation
        try {
            $n = $client.Client.ReceiveMessageFrom($buffer, 0, $buffer.Length, [ref]$flags, [ref]$remote, [ref]$info)
        } catch [System.Net.Sockets.SocketException] {
            continue   # Zeitlimit von 1 s, weiter warten
        }
        $data = New-Object byte[] $n
        [Array]::Copy($buffer, $data, $n)
        $count++
        $hex = ($data | ForEach-Object { $_.ToString('x2') }) -join ''
        $head = if ($data.Length -ge 4) {
            'ver={0} cmd=0x{1} len={2}' -f $data[0], $data[1].ToString('x2'), ($data[2] * 256 + $data[3])
        } else { 'kurz' }
        $line = '{0} from={1} to={2} bytes={3} {4} hex={5}' -f (Get-Date -Format o), $remote, $info.Address, $data.Length, $head, $hex
        $line | Out-File -Encoding ascii -Append $OutFile
        Write-Host $line.Substring(0, [Math]::Min(160, $line.Length))
    }
} finally {
    $client.Close()
}
"# Ende $(Get-Date -Format o), $count Datagramme" | Out-File -Encoding ascii -Append $OutFile
Write-Host "Fertig: $count Datagramme -> $OutFile"
