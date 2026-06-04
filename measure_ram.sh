#!/bin/bash
# Measures RAM of rustployd and rustploy.
# Client TUI runs in the FOREGROUND (owns the terminal).
# All measurements go silently to /tmp/rustploy_ram.log.
#
# Usage: ./measure_ram.sh [interval_seconds]
# Watch live in another terminal: tail -f /tmp/rustploy_ram.log

set -euo pipefail

INTERVAL=${1:-2}
DAEMON_BIN="./target/release/rustployd"
CLIENT_BIN="./target/release/rustploy"
LOG="/tmp/rustploy_ram.log"
SOCKET=${RUSTPLOY_SOCKET_PATH:-/run/rustploy/rustploy.sock}

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

DAEMON_PID=""
MEASURE_PID=""

print_summary() {
    [[ -f "$LOG" ]] && [[ $(wc -l < "$LOG") -gt 1 ]] || return
    echo ""
    echo -e "${CYAN}=== RAM log: $LOG ===${NC}"
    awk 'NR==1 {
        printf "%-10s  %13s  %13s  %13s  %13s\n", $1,$2,$3,$4,$5; next
    }
    { printf "%-10s  %10.1f MB  %10.1f MB  %10.1f MB  %10.1f MB\n",
        $1, $2/1024, $3/1024, $4/1024, $5/1024 }' "$LOG"
    MAX_D=$(awk 'NR>1{print $2}' "$LOG" | sort -n | tail -1)
    MAX_C=$(awk 'NR>1{print $4}' "$LOG" | sort -n | tail -1)
    echo ""
    echo -e "${CYAN}Pico RSS daemon: ${YELLOW}$(awk "BEGIN{printf \"%.1f MB\", ${MAX_D:-0}/1024}")${NC}"
    echo -e "${CYAN}Pico RSS client: ${YELLOW}$(awk "BEGIN{printf \"%.1f MB\", ${MAX_C:-0}/1024}")${NC}"
}

cleanup() {
    [[ -n "$MEASURE_PID" ]] && kill "$MEASURE_PID" 2>/dev/null || true
    [[ -n "$DAEMON_PID"  ]] && kill "$DAEMON_PID"  2>/dev/null || true
    wait 2>/dev/null || true
    print_summary
}
trap cleanup EXIT INT TERM

ram_kib() { grep -m1 "^VmRSS:"  "/proc/$1/status" 2>/dev/null | awk '{print $2}' || echo 0; }
vsz_kib() { grep -m1 "^VmSize:" "/proc/$1/status" 2>/dev/null | awk '{print $2}' || echo 0; }

# measure_loop: finds client PID dynamically via pgrep each iteration
measure_loop() {
    local dpid=$1
    printf "%-10s %14s %14s %14s %14s\n" \
        "timestamp" "daemon_rss_kib" "daemon_vsz_kib" "client_rss_kib" "client_vsz_kib" > "$LOG"
    while true; do
        kill -0 "$dpid" 2>/dev/null || break
        local cpid
        cpid=$(pgrep -x rustploy 2>/dev/null | head -1 || echo 0)
        printf "%-10s %14s %14s %14s %14s\n" \
            "$(date '+%H:%M:%S')" \
            "$(ram_kib "$dpid")" "$(vsz_kib "$dpid")" \
            "$(ram_kib "$cpid")" "$(vsz_kib "$cpid")" >> "$LOG"
        sleep "$INTERVAL"
    done
}

# --- Build if needed ---
if [[ ! -x "$DAEMON_BIN" || ! -x "$CLIENT_BIN" ]]; then
    echo -e "${GREEN}Compilando...${NC}"
    cargo build --release -p daemon -p client
fi

# --- Start daemon in background ---
sudo mkdir -p /run/rustploy
"$DAEMON_BIN" &>>/tmp/rustployd.log &
DAEMON_PID=$!
echo -n "Aguardando daemon (PID $DAEMON_PID)"
for i in $(seq 1 20); do
    [[ -S "$SOCKET" ]] && { echo -e " ${GREEN}ok${NC}"; break; }
    sleep 0.5; echo -n "."
    if [[ $i -eq 20 ]]; then
        echo -e " ${RED}timeout — verifique /tmp/rustployd.log${NC}"; exit 1
    fi
done

# --- Start measure loop in background (writes only to file, never to stdout) ---
measure_loop "$DAEMON_PID" &>/dev/null &
MEASURE_PID=$!

echo -e "Log de RAM: ${YELLOW}$LOG${NC}  |  Daemon log: ${YELLOW}/tmp/rustployd.log${NC}"
echo -e "(Outro terminal: ${CYAN}tail -f $LOG${NC})"
echo ""

# --- Run client in FOREGROUND so it fully owns the terminal ---
"$CLIENT_BIN"
