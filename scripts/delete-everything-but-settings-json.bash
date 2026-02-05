#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="./DrakonixAnvilData"

echo "This will DELETE everything in $DATA_DIR/ except settings.json:"
echo "  - All server configs (servers.json)"
echo "  - All server data (worlds, mods, configs)"
echo "  - All backups"
echo "  - All logs"
echo ""

read -p "Type 'yes' to confirm: " confirm
if [[ "$confirm" != "yes" ]]; then
    echo "Aborted."
    exit 1
fi

if [[ ! -d "$DATA_DIR" ]]; then
    echo "No $DATA_DIR/ directory found, nothing to delete."
    exit 0
fi

# Back up settings, wipe, restore
if [[ -f "$DATA_DIR/settings.json" ]]; then
    cp "$DATA_DIR/settings.json" /tmp/drakonix_settings_backup.json
fi

rm -rf "${DATA_DIR:?}"/*

if [[ -f /tmp/drakonix_settings_backup.json ]]; then
    mv /tmp/drakonix_settings_backup.json "$DATA_DIR/settings.json"
fi

echo "Done. $DATA_DIR/ wiped (settings.json preserved)."
