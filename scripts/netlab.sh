#!/usr/bin/env bash
# Synthetisches Netzwerk-Lab (AKL-M080-01): misst je Docker-Netzmodus
# (host, bridge, macvlan), ob ein UDP-Broadcast aus dem "LAN" im Container
# ankommt, ob ein TCP-Port des Containers aus dem LAN erreichbar ist und mit
# welcher Absenderadresse der Container die Gegenstelle sieht.
#
# Aufbau: eigene Linux-Bridge br-akllab (10.77.0.1/24) als LAN-Switch; das
# "LAN-Gerät" ist ein Container ohne Docker-Netz, per veth an die Bridge
# gehängt (10.77.0.50). macvlan hängt an einer zweiten veth (wie eine
# Netzwerkkarte am Switch). Protokollneutral, nur akl-netprobe; kein Verkehr
# außerhalb der Bridge.
#
# Voraussetzung: Linux als root (ip, nsenter), Docker, Image mit akl-netprobe
# (scripts/docker-build.sh). Unter Docker Desktop: scripts\lab.ps1 netlab.
set -euo pipefail

IMAGE=${AKL_IMAGE:-ai-keyless:dev}
NET=10.77.0
BR=br-akllab
UDP=47001
TCP=47002
P=akl-netlab
LAN_IP=$NET.50
MV_IP=$NET.130

cleanup() {
  docker rm -f "$P-probe" "$P-lan" >/dev/null 2>&1 || true
  docker network rm "$P-bridge" "$P-macvlan" >/dev/null 2>&1 || true
  ip link del akl-mv0 2>/dev/null || true
  ip link del akl-lan-br 2>/dev/null || true
  ip link del "$BR" 2>/dev/null || true
}
trap cleanup EXIT
cleanup

ip link add "$BR" type bridge
ip addr add "$NET.1/24" dev "$BR"
ip link set "$BR" up

# LAN-Gerät: läuft als Listener, damit der Container am Leben bleibt.
docker run -d --name "$P-lan" --network none --entrypoint akl-netprobe "$IMAGE" \
  listen --tcp 47099 --seconds 3600 >/dev/null
pid=$(docker inspect -f '{{.State.Pid}}' "$P-lan")
ip link add akl-lan0 type veth peer name akl-lan-br
ip link set akl-lan-br master "$BR" up
ip link set akl-lan0 netns "$pid"
nsenter -t "$pid" -n ip link set akl-lan0 name eth0
nsenter -t "$pid" -n ip addr add "$LAN_IP/24" brd + dev eth0
nsenter -t "$pid" -n ip link set eth0 up
nsenter -t "$pid" -n ip route add default via "$NET.1"

ip link add akl-mv0 type veth peer name akl-mv-br
ip link set akl-mv-br master "$BR" up
ip link set akl-mv0 up

docker network create "$P-bridge" >/dev/null
docker network create -d macvlan --subnet "$NET.0/24" --gateway "$NET.1" \
  --ip-range "$NET.128/25" -o parent=akl-mv0 "$P-macvlan" >/dev/null

lan() { docker exec "$P-lan" akl-netprobe "$@"; }
mark() { if [ "$1" = 1 ]; then echo "✓"; else echo "✗"; fi; }

# measure MODUS ZIEL-IP [docker-run-Argumente …]
measure() {
  local mode=$1 target=$2
  shift 2
  docker run -d --name "$P-probe" "$@" --entrypoint akl-netprobe "$IMAGE" \
    listen --udp "$UDP" --tcp "$TCP" --seconds 30 >/dev/null
  sleep 1
  # Leitet ein Userland-Proxy veröffentlichte Ports weiter? (Linux-Standard)
  local proxies
  proxies=$(pgrep -c -x docker-proxy || true)
  lan send "$NET.255:$UDP" --count 3 --tag subnet >/dev/null || true
  lan send "255.255.255.255:$UDP" --count 3 --tag global >/dev/null || true
  local conn seen="–" tcp=0
  if conn=$(lan connect "$target:$TCP" --timeout 3 2>&1); then
    tcp=1
    seen=${conn##*seen_as=}
  fi
  sleep 0.5
  local logs sub=0 glob=0 udp_from
  logs=$(docker logs "$P-probe" 2>&1)
  grep -q 'payload=AKL-NETPROBE subnet' <<<"$logs" && sub=1
  grep -q 'payload=AKL-NETPROBE global' <<<"$logs" && glob=1
  # Absender des ersten empfangenen Broadcasts (IP ohne Port).
  udp_from=$(grep -m1 -o 'udp from=[0-9.]*' <<<"$logs" | cut -d= -f2 || true)
  docker rm -f "$P-probe" >/dev/null
  printf '| %s | %s | %s | %s | %s (%s:%s) | %s | %s |\n' "$mode" "$(mark $sub)" "$(mark $glob)" \
    "${udp_from:-–}" "$(mark $tcp)" "$target" "$TCP" "${seen%:*}" "${proxies:-0}"
  [ "$mode" != host ] || [ "$sub$glob$tcp" = 111 ] || HOST_FAIL=1
}

HOST_FAIL=0
echo "Netzwerk-Lab $(date -u +%FT%TZ), Image $IMAGE, LAN-Gerät $LAN_IP"
echo
echo "| Modus | Broadcast $NET.255 | Broadcast 255.255.255.255 | UDP-Absender laut Container | TCP aus dem LAN | TCP-Absender laut Container | docker-proxy-Prozesse |"
echo "|---|---|---|---|---|---|---|"
measure host "$NET.1" --network host
measure bridge "$NET.1" --network "$P-bridge" -p "$UDP:$UDP/udp" -p "$TCP:$TCP/tcp"
measure macvlan "$MV_IP" --network "$P-macvlan" --ip "$MV_IP"

# host muss alles empfangen – sonst ist der Laboraufbau selbst defekt.
if [ "$HOST_FAIL" = 1 ]; then
  echo "FEHLER: host-Modus empfängt nicht alles – Laboraufbau prüfen." >&2
  exit 1
fi
