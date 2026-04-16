# CurseForge API Partner Requirement

## Finding

The CurseForge `/v1/mods/search` endpoint returns `403 Forbidden` with a standard free API key,
even when other endpoints work fine with the same key.

Confirmed via `test-cf-api` binary (see `src/bin/test-cf-api.rs`):

| Endpoint              | Status |
|-----------------------|--------|
| `GET /v1/games`       | 200 OK |
| `GET /v1/mods/search` | 403    |
| `GET /v1/mods/{id}`   | 200 OK |

## Implication

The in-app CurseForge modpack browser (`src/ui/cf_browse.rs`) requires search access and will
always fail for users with a standard free API key.

## Options

1. **Apply for partner/expanded API access** at https://console.curseforge.com/ — submit an
   application describing the use case (self-hosted Minecraft server manager).
2. **Replace browse with direct entry** — remove the search UI and have users paste a CurseForge
   slug or project ID directly. Direct mod lookups (`GET /v1/mods/{id}`) work with a free key.
3. **Use FTB API for FTB packs** — FTB packs can be installed via `TYPE=FTBA` without any CF key.
   Reserve CF integration for non-FTB CurseForge packs only.
