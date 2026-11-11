#!/usr/bin/env bash
# In-game check for the Comforts sleeping-bag port (slice 1), on the campaign
# pin: one `.hnmod` installs the content and the behaviour.
#
# Legs:
#  EMBEDDED=1 (default)  the game is launched with **no `--dev-mod`**: the bag
#                        comes from the bundle's embedded copy
#                        (`mixins/comforts/data/comforts-content.hmod`, the AE2
#                        pattern), which the helper writes to `.horizon-cache/`.
#                        The run checks that file is byte-identical to
#                        `content/comforts.hmod`.
#  CONTROL=1 (default with EMBEDDED)
#                        the same binary first with `.horizon-cache` unusable
#                        (a plain file), where the helper cannot write its
#                        content and both ids must be unknown — otherwise the
#                        embedded leg proves nothing.
#  DEV=1                 the game loads `content/` as a dev mod (the split-port
#                        route, kept as a diagnostic).
#
# The scenario, once the bag is known:
#  A. Night sleep in the bag skips the night (the clock reaches the next
#     morning) and the player wakes up again.
#  B. That sleep does NOT set the respawn point: no "Respawn point set" line,
#     no `respawn` in the player's NBT, and after a death the player respawns at
#     the world spawn, not at the bag.
#  C. The vanilla bed control DOES set the spawn: the same night sleep shows the
#     message, the NBT carries `respawn`, and the death respawns at the bed. An
#     implementation that ignored `player_spawn_point_set` fails B while C still
#     passes — so the scenario is fail-capable.
#  D. A daytime sleep in the bag is refused with the default `sleepingBagUse`
#     ("NIGHT"); with `sleepingBagUse = "DAY"` the same bag sleeps by day and
#     refuses at night (the config is live, not a constant).
#  D2. `restrictSleeping = true` refuses a second sleep right after the first;
#     with it false the same click sleeps again (the config again).
#  E. `sleepingBagBreakChance = 100` makes the bag break on waking (both halves
#     gone, nothing dropped, as upstream's `removeBlock`); the bag from A
#     survived its night at the default 0, which is the differential.
#  F. Breaking the bag by hand returns one item (the 26.3 loot table's head-only
#     self-drop).
#
#   tools/e2e.sh                  # embedded leg + control + the scenario
#   DEV=1 tools/e2e.sh            # the same scenario over --dev-mod content/
#   SELFTEST=1 tools/e2e.sh       # corrupt expectation: the bag MUST have set a
#                                 # spawn, which is impossible; exits 1, proving
#                                 # B can fail
#
# Harness notes: this port registers no commands, so `ctl command` is used
# rather than `ctl chat "/cmd"`. The world is survival (loot tables and mining
# behave as for a player) on a stone floor at y=63; `ctl place` places against a
# neighbour face, so the player stands two blocks west and aims at the floor.
# A bed sets `part=head`/`part=foot`, so every check finds the halves by reading
# the block state instead of assuming the placement direction.
set -Eeuo pipefail

CLONE=${CLONE:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}}
PORT=$CLONE/ports/comforts
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/comforts-e2e.sock}
EMBEDDED=${EMBEDDED:-1}
DEV=${DEV:-0}
if [ "$DEV" = 1 ]; then EMBEDDED=0; fi
CONTROL=${CONTROL:-$EMBEDDED}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
CTL=${CTL:-$PORT/target/horizon-cargo/fast/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
CONTENT=${CONTENT:-$PORT/content}
EVIDENCE=$CLONE/ports/_shared/evidence/comforts
LEG=$([ "$EMBEDDED" = 1 ] && echo embedded || echo dev)
[ "$SELFTEST" = 1 ] && LEG=$LEG-selftest
LOG=${LOG:-$EVIDENCE/e2e-$LEG.log}

mkdir -p "$EVIDENCE" "$(dirname "$LOG")"
# The log is the evidence: everything the run says goes to it, and the last line
# is the exit status `check-e2e-logs.py` reads.
exec > >(tee "$LOG") 2>&1

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$CONTENT/horizon-mod.toml" ] || { echo "missing content mod: $CONTENT" >&2; exit 1; }

# The control client and the game come from the port's own build (the slot's
# `target/horizon-cargo`), so they match the pin and the sources. Build them
# here when a fresh lease has not yet.
if [ ! -x "$CTL" ] || [ ! -x "$GAME" ]; then
    echo "== building the game and the control client (one lease) =="
    ( cd "$PORT" && python3 tools/horizon.py ctl >/dev/null && python3 tools/horizon.py build >/dev/null )
fi
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/comforts-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"
# The game's folder is the working directory (its `config/`, `saves/` and
# `.horizon-cache/` live beside it), so the run never touches player data.
cd "$SCRATCH"
CONFIG=$SCRATCH/config/comforts-server.toml
mkdir -p "$SCRATCH/config"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 40 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$status"
}
trap cleanup EXIT

write_config() { # write_config [extra line ...]
    {
        echo '# Comforts server rules for the e2e.'
        printf 'sleepingBagUse = "NIGHT"\n'
        printf 'daySleepingPercentage = -1\n'
        printf 'dayWakeTimeOffset = 0\n'
        printf 'nightWakeTimeOffset = 0\n'
        printf 'restrictSleeping = false\n'
        printf 'sleepingBagsStopPhantoms = true\n'
        printf 'sleepingBagBreakChance = 0\n'
        for line in "$@"; do printf '%s\n' "$line"; done
    } > "$CONFIG"
}
write_config

launch() { # launch <world>
    local world=$1
    rm -f "$SOCK"
    local args=(--game "$GAME" --world "$world" --seed 20261106 --headless --wait
                --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        echo "  launch: --dev-mod $CONTENT" >&2
        args+=(--dev-mod "$CONTENT")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
    [ "$(ctl state | jq -r '.screen')" = "playing" ] || fail "the game is not playing after launch"
}

state() { ctl state; }
sleeping_pos() { state | jq -c '.sleeping'; }
time_of_day() { state | jq -r '.time.time_of_day'; }
day_time() { state | jq -r '.time.day_time'; }
position() { state | jq -c '.player.position'; }
chat_text() { ctl messages 60 | jq -r '.chat | join("\n")'; }
player_nbt() { ctl command "data get entity @s" 2>&1; }
wait_ticks() { ctl wait "$1" >/dev/null; }
block_of() { ctl block "$1" "$2" "$3" | jq -r '.block // ""'; }
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // ""'; }
# How many sleeping bags lie on the floor (dropped items), summed over stacks.
bag_drops() {
    ctl entities 12 item \
        | jq '[.entities[] | select(.item.item == "comforts:sleeping_bed") | .item.count] | add // 0'
}

arena() {
    ctl command "gamemode survival"
    ctl command "difficulty peaceful"
    ctl command "weather clear"
    ctl command "kill @e[type=!player]"
    # A world-spawn respawn otherwise lands on a random column within the
    # `respawn_radius` (vanilla's PlayerSpawnFinder); 0 makes "respawned at the
    # world spawn" an exact, fail-capable assertion.
    ctl command "gamerule respawn_radius 0"
    # Build the arena around wherever the player spawned, so the run never
    # depends on the seed's terrain height: a stone floor one block below the
    # feet and six blocks of air above, then the world spawn on it.
    local pos
    pos=$(position)
    AX=$(printf '%s' "$pos" | jq -r '.[0] | floor')
    AY=$(printf '%s' "$pos" | jq -r '.[1] | floor')
    AZ=$(printf '%s' "$pos" | jq -r '.[2] | floor')
    WSX=$((AX))
    WSZ=$((AZ))
    ctl fill $((AX - 8)) $((AY - 1)) $((AZ - 6)) $((AX + 8)) $((AY - 1)) $((AZ + 10)) minecraft:stone
    ctl fill $((AX - 8)) $((AY)) $((AZ - 6)) $((AX + 8)) $((AY + 6)) $((AZ + 10)) minecraft:air
    ctl command "kill @e[type=!player]"
    ctl command "setworldspawn $AX $AY $AZ"
    ctl tp "$AX" "$AY" "$AZ"
    wait_ticks 20
    [ "$(ctl state | jq -r '.screen')" = "playing" ] || fail "the screen is not playing"
    [ "$(block_of "$AX" $((AY - 1)) "$AZ")" = "minecraft:stone" ] \
        || fail "the arena floor is not stone"
    [ "$(block_of "$AX" "$AY" "$AZ")" = "minecraft:air" ] \
        || fail "the arena air above the floor is not air"
    echo "  arena at $AX $AY $AZ (world spawn $WSX $WSZ)" >&2
}

bag_positions() {
    ctl find-block comforts:sleeping_bed 12 \
        | jq -r '.nearest[].pos | "\(.[0]) \(.[1]) \(.[2])"'
}
part_of() { # part_of <x> <y> <z> -> head|foot|""
    block_state "$1" "$2" "$3" | grep -o 'part=[a-z]*' | head -1 | cut -d= -f2
}
bag_half() { # bag_half <head|foot>
    local want=$1 x y z
    while read -r x y z; do
        [ -n "$x" ] || continue
        if [ "$(part_of "$x" "$y" "$z")" = "$want" ]; then
            echo "$x $y $z"
            return 0
        fi
    done < <(bag_positions)
    return 1
}
head_of_bag() { bag_half head; }
foot_of_bag() { bag_half foot; }

place_bed() { # place_bed <item> <x> <z>
    local item=$1 x=$2 z=$3
    ctl tp "$x" "$AY" $((z - 2)) >/dev/null
    wait_ticks 10
    ctl give "$item" 1 >/dev/null 2>&1 || true
    ctl look-at-block "$x" $((AY - 1)) "$z" up >/dev/null 2>&1 || ctl look 0 20 >/dev/null
    ctl place "$x" "$AY" "$z" --item "$item" >/dev/null
    wait_ticks 10
}

sleep_in_bag() { # sleep_in_bag <label>: sleeps in the nearest bag
    local label=$1 head hx hz sleeping
    head=$(head_of_bag) || fail "$label: no sleeping bag head found"
    hx=$(echo "$head" | cut -d' ' -f1)
    hz=$(echo "$head" | cut -d' ' -f3)
    ctl tp "$hx" "$AY" $((hz - 2)) >/dev/null
    wait_ticks 6
    ctl interact $head >/dev/null
    wait_ticks 6
    sleeping=$(sleeping_pos)
    [ "$sleeping" != "null" ] || fail "$label: did not fall asleep (sleeping=$sleeping)"
    echo "  $label: asleep at $sleeping (head $head)" >&2
}

# Let a sleep run to its own end. A bed's wake-up is the engine's (enough
# sleepers, 100 ticks deep), so the scenario never has to fake one; a single
# player's Shift cannot wake it (the server's sleep tick only wakes a dead
# sleeper or one whose bed is gone).
finish_sleep() { # finish_sleep <label>
    local label=$1 i
    for i in $(seq 1 24); do
        wait_ticks 10
        [ "$(sleeping_pos)" = "null" ] && break
    done
    [ "$(sleeping_pos)" = "null" ] || fail "$label: still asleep after 240 ticks"
    echo "  $label: woke, day_time $(day_time), time_of_day $(time_of_day)" >&2
}

wait_for_morning() { # wait_for_morning <label> <before day_time>
    local label=$1 before=$2 i
    for i in $(seq 1 24); do
        wait_ticks 10
        [ "$(sleeping_pos)" = "null" ] && break
    done
    [ "$(sleeping_pos)" = "null" ] || fail "$label: still asleep after 240 ticks"
    local after tod
    after=$(day_time)
    tod=$(time_of_day)
    echo "  $label: woke, day_time $before -> $after, time_of_day $tod" >&2
    awk -v a="$after" -v b="$before" 'BEGIN { exit !(a > b && a < b + 12000) }' \
        || fail "$label: the night was not skipped (day_time $before -> $after)"
    awk -v t="$tod" 'BEGIN { exit !(t >= 0 && t < 400) }' \
        || fail "$label: woke at time_of_day $tod, not the morning"
}

distance() { # distance <x1> <z1> <x2> <z2>
    awk -v ax="$1" -v az="$2" -v bx="$3" -v bz="$4" \
        'BEGIN { dx = ax - bx; dz = az - bz; printf "%.2f", sqrt(dx * dx + dz * dz) }'
}

die_and_respawn() {
    ctl command "kill @s"
    wait_ticks 20
    ctl respawn >/dev/null
    wait_ticks 20
    position
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. Both ids must then be unknown; if they resolve, the run proves
# nothing about the embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch ComfortsControl
    ctl command "kill @e[type=!player]" >/dev/null || true
    local give set name
    give=$(ctl command "item replace entity @s hotbar.0 with comforts:sleeping_bed 1" 2>&1 || true)
    case "$give" in
        *"Unknown item 'comforts:sleeping_bed'"*) ;;
        *) fail "the control game knows the bag item, so the content did not come from the embedded file: $give" ;;
    esac
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "comforts:sleeping_bed")] | length == 0' >/dev/null \
        || fail "the control game has the bag item, so the content did not come from the embedded file"
    set=$(ctl command "setblock 2 64 0 comforts:sleeping_bed" 2>&1 || true)
    case "$set" in
        *"Unknown block type 'comforts:sleeping_bed'"*) ;;
        *) fail "the control game knows the bag block, so the content did not come from the embedded file: $set" ;;
    esac
    name=$(block_of 2 64 0)
    [ "$name" != "comforts:sleeping_bed" ] || fail "the control game holds the bag block"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: 'Unknown item' and 'Unknown block type', the block at 2 64 0 is $name" >&2
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -f "$SCRATCH/.horizon-cache"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/comforts-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/comforts.hmod" \
        || fail "the embedded content differs from content/comforts.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/comforts.hmod"
}

echo "== launch (Overworld arena, $([ "$EMBEDDED" = 1 ] && echo 'bundle only' || echo 'dev content')) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch ComfortsE2E
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

echo "== content: the bag is a block and an item =="
ctl give comforts:sleeping_bed 6 >/dev/null || fail "cannot give comforts:sleeping_bed"
ctl give minecraft:white_bed 2 >/dev/null || fail "cannot give the control bed"
ctl give minecraft:stone_pickaxe 1 >/dev/null || fail "cannot give the pickaxe"
wait_ticks 5
inv=$(ctl inventory)
bags=$(printf '%s' "$inv" | jq -r '.counts["comforts:sleeping_bed"] // 0')
[ "$bags" -ge 6 ] || fail "comforts:sleeping_bed is missing from the inventory"
echo "  inventory: comforts:sleeping_bed=$bags" >&2

echo "== A/B: night sleep in the bag skips the night and sets no spawn =="
place_bed comforts:sleeping_bed "$AX" $((AZ + 7))
head=$(head_of_bag) || fail "the placed bag has no head half"
foot=$(foot_of_bag) || fail "the placed bag has no foot half"
[ "$(block_of $head)" = "comforts:sleeping_bed" ] || fail "the head is not a comforts bed"
echo "  placed: foot $foot, head $head" >&2
ctl command "time set 18000"
wait_ticks 5
before=$(day_time)
sleep_in_bag "bag"
wait_for_morning "bag" "$before"
bag_head=$head
bag_foot=$foot

log=$(chat_text)
case "$log" in
    *"Respawn point set"*) fail "the sleeping bag set a respawn point: $log" ;;
esac
echo "  no 'Respawn point set' line in the chat log" >&2

nbt=$(player_nbt)
echo "  player NBT respawn read: $(printf '%s' "$nbt" | head -c 240)" >&2
case "$nbt" in
    *respawn*) fail "the sleeping bag wrote a respawn point: $nbt" ;;
esac
if [ "$SELFTEST" = "1" ]; then
    fail "SELFTEST: expected the bag's NBT to carry a respawn point; it does not, so assertion B can fail (this exit 1 is the proof)"
fi

respawn_pos=$(die_and_respawn)
rx=$(printf '%s' "$respawn_pos" | jq -r '.[0]')
rz=$(printf '%s' "$respawn_pos" | jq -r '.[2]')
hx=$(echo "$head" | cut -d' ' -f1)
hz=$(echo "$head" | cut -d' ' -f3)
to_spawn=$(distance "$rx" "$rz" "$WSX" "$WSZ")
to_bag=$(distance "$rx" "$rz" "$hx" "$hz")
echo "  respawned at $respawn_pos: $to_spawn from the world spawn, $to_bag from the bag" >&2
awk -v d="$to_spawn" 'BEGIN { exit !(d <= 2.0) }' \
    || fail "respawn was not at the world spawn ($respawn_pos)"
awk -v d="$to_bag" 'BEGIN { exit !(d > 3.0) }' \
    || fail "respawn used the sleeping bag as the spawn point ($respawn_pos)"

[ "$(block_of $bag_head)" = "comforts:sleeping_bed" ] \
    || fail "the bag disappeared after a night at break chance 0"
echo "  the bag survived the night (break chance 0)" >&2

echo "== C: the vanilla bed control does set the spawn =="
place_bed minecraft:white_bed $((AX + 4)) $((AZ + 4))
bed_head=$(ctl find-block minecraft:white_bed 12 \
    | jq -r --argjson y "$AY" '.nearest[] | select(.pos[1] == $y) | .pos | "\(.[0]) \(.[1]) \(.[2])"' | head -1)
[ -n "$bed_head" ] || fail "the vanilla bed did not place"
ctl command "time set 18000"
wait_ticks 5
before=$(day_time)
bhx=$(echo "$bed_head" | cut -d' ' -f1)
bhz=$(echo "$bed_head" | cut -d' ' -f3)
ctl tp "$bhx" "$AY" $((bhz - 2)) >/dev/null
wait_ticks 6
ctl interact $bed_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" != "null" ] || fail "the vanilla bed control did not put the player to sleep"
wait_for_morning "vanilla bed" "$before"
log=$(chat_text)
case "$log" in
    *"Respawn point set"*) echo "  'Respawn point set' seen (vanilla bed)" >&2 ;;
    *) fail "the vanilla bed did not report a respawn point: $log" ;;
esac
nbt=$(player_nbt)
case "$nbt" in
    *respawn*) echo "  the player NBT carries a respawn point (vanilla bed)" >&2 ;;
    *) fail "the vanilla bed wrote no respawn point: $nbt" ;;
esac
respawn_pos=$(die_and_respawn)
rx=$(printf '%s' "$respawn_pos" | jq -r '.[0]')
rz=$(printf '%s' "$respawn_pos" | jq -r '.[2]')
to_bed=$(distance "$rx" "$rz" "$bhx" "$bhz")
to_spawn=$(distance "$rx" "$rz" "$WSX" "$WSZ")
echo "  respawned at $respawn_pos: $to_bed from the bed, $to_spawn from the world spawn" >&2
awk -v d="$to_bed" 'BEGIN { exit !(d <= 3.0) }' \
    || fail "the vanilla bed did not become the respawn point ($respawn_pos)"
awk -v d="$to_spawn" 'BEGIN { exit !(d > 2.0) }' \
    || fail "the vanilla bed control respawned at the world spawn ($respawn_pos)"

echo "== D: daytime sleep is refused (NIGHT), and the config is live =="
place_bed comforts:sleeping_bed $((AX - 4)) $((AZ + 4))
day_head=$(head_of_bag) || fail "the daytime bag has no head half"
ctl command "time set 6000"
wait_ticks 5
ctl tp $((AX - 4)) "$AY" $((AZ + 2)) >/dev/null
wait_ticks 5
ctl interact $day_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" = "null" ] || fail "the bag let the player sleep at noon"
echo "  noon with sleepingBagUse=NIGHT: refused (actionbar '$(ctl messages 5 | jq -r '.actionbar // ""')')" >&2

echo "  -- sleepingBagUse = DAY (config reload) --" >&2
write_config 'sleepingBagUse = "DAY"'
wait_ticks 60
ctl command "time set 6000"
wait_ticks 5
ctl interact $day_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" != "null" ] || fail "sleepingBagUse=DAY did not allow a daytime sleep"
echo "  noon with sleepingBagUse=DAY: asleep" >&2
finish_sleep "day sleep"
# Upstream's `getWakeTime` for a day sleeper: the coming dusk (11 999), not the
# next morning. The night offset is 0 here, so the clock lands exactly there.
tod=$(time_of_day)
awk -v t="$tod" 'BEGIN { exit !(t >= 11990 && t <= 12010) }' \
    || fail "a day sleep woke at time_of_day $tod, not the dusk (11999)"
echo "  the day sleep skipped to the dusk (time_of_day $tod)" >&2

ctl command "time set 18000"
wait_ticks 5
ctl interact $day_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" = "null" ] || fail "sleepingBagUse=DAY let the player sleep at night"
echo "  night with sleepingBagUse=DAY: refused (the config drives the window)" >&2

echo "== D2: restrictSleeping refuses the next sleep, and only while set =="
write_config
wait_ticks 40
ctl tp $((AX - 4)) "$AY" $((AZ + 2)) >/dev/null
ctl command "time set 18000"
wait_ticks 5
before=$(day_time)
sleep_in_bag "restrict control"
wait_for_morning "restrict control" "$before"
echo "  after a full night the player is tired: a second sleep should be refused" >&2
ctl command "time set 18000"
wait_ticks 5
write_config 'restrictSleeping = true'
wait_ticks 60
ctl interact $day_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" = "null" ] || fail "restrictSleeping=true allowed a sleep while tired"
echo "  restrictSleeping=true: refused (actionbar '$(ctl messages 5 | jq -r '.actionbar // ""')')" >&2
write_config
wait_ticks 60
ctl interact $day_head >/dev/null
wait_ticks 6
[ "$(sleeping_pos)" != "null" ] || fail "restrictSleeping=false still refused the tired sleep"
echo "  restrictSleeping=false: the same click slept" >&2
finish_sleep "restricted-off sleep"

echo "== E: sleepingBagBreakChance = 100 breaks the bag on waking =="
# The earlier deaths dropped the inventory on the floor; clear it so "the bag
# dropped nothing" counts only what this sleep produces.
ctl command "kill @e[type=item]" >/dev/null
wait_ticks 10
bag_drops_before=$(bag_drops)
echo "  bag item entities on the floor before the sleep: $bag_drops_before" >&2
write_config 'sleepingBagBreakChance = 100'
wait_ticks 60
ctl tp $((AX - 4)) "$AY" $((AZ + 2)) >/dev/null
ctl command "time set 18000"
wait_ticks 5
before=$(day_time)
sleep_in_bag "break test"
wait_for_morning "break test" "$before"
[ "$(block_of $day_head)" = "minecraft:air" ] \
    || fail "the bag did not break at sleepingBagBreakChance=100"
echo "  the bag broke on waking (block at $day_head is air)" >&2
bag_drops_after=$(bag_drops)
echo "  bag item entities on the floor after the sleep: $bag_drops_after" >&2
[ "$bag_drops_after" -eq "$bag_drops_before" ] \
    || fail "the broken bag dropped items ($bag_drops_before -> $bag_drops_after bag items on the floor)"
echo "  the broken bag dropped nothing (as upstream's removeBlock)" >&2

echo "== F: breaking the bag by hand returns one item =="
write_config
wait_ticks 60
ctl tp "$AX" "$AY" $((AZ + 5)) >/dev/null
wait_ticks 5
before_bags=$(ctl inventory | jq -r '.counts["comforts:sleeping_bed"] // 0')
bhx=$(echo "$bag_head" | cut -d' ' -f1)
bhy=$(echo "$bag_head" | cut -d' ' -f2)
bhz=$(echo "$bag_head" | cut -d' ' -f3)
ctl break "$bhx" "$bhy" "$bhz" >/dev/null
wait_ticks 10
grounded=$(ctl entities 6 item)
case "$grounded" in
    *comforts:sleeping_bed*) echo "  the head dropped a bag on the ground" >&2 ;;
    *) fail "breaking the bag dropped nothing: $grounded" ;;
esac
ctl tp "$bhx" "$AY" "$bhz" >/dev/null
wait_ticks 30
after_bags=$(ctl inventory | jq -r '.counts["comforts:sleeping_bed"] // 0')
echo "  bags in the inventory: $before_bags -> $after_bags" >&2
[ "$after_bags" -gt "$before_bags" ] || fail "breaking the bag returned no item"
[ "$(block_of $bag_head)" = "minecraft:air" ] || fail "breaking the head left the block"
# The pin removes a bed's other half only in creative (hooks.rs:459), so a
# survival break leaves the foot standing; the port reports that as a pin gap
# and finishes it off here (the foot drops nothing: head-only loot table).
if [ "$(block_of $bag_foot)" = "comforts:sleeping_bed" ]; then
    echo "  the pin left the foot half standing after the head broke (pin gap)" >&2
    ctl break $(echo "$bag_foot" | cut -d' ' -f1) $(echo "$bag_foot" | cut -d' ' -f2) \
        $(echo "$bag_foot" | cut -d' ' -f3) >/dev/null
    wait_ticks 10
fi
[ "$(block_of $bag_foot)" = "minecraft:air" ] || fail "breaking the foot left the block"

echo "PASS: night sleep in the bag skipped the night with no spawn point set (world spawn on death, no 'Respawn point set', no NBT respawn); the vanilla bed control set one and respawned there; a daytime sleep was refused at NIGHT and granted at DAY, refused again at night; restrictSleeping refused a second sleep only while set; break chance 100 broke the bag with no drop while the chance-0 bag survived; breaking the bag returned one item"
exit 0
