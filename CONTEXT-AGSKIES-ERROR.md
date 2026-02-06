# Agrarian Skies 2 Server Setup - Issue Tracker

## Problem History

### 1. Java Version Mismatch (FIXED - commit 879aeba)
Forge 1.7.10 requires Java 8. The itzg image was using Java 21+ (`:latest` tag).
`ClassCastException: AppClassLoader cannot be cast to URLClassLoader` — Java 9+
removed URLClassLoader from the app class loader.
**Fix:** Added per-server `java_version` field, map to Docker image tags (`:java8`).

### 2. Client-Only Mod Crashes (ROOT CAUSE FOUND)
Using `TYPE=AUTO_CURSEFORGE` downloads the **client manifest** which includes
client-only mods that crash on dedicated servers:
- `Resource Loader` → `ClassNotFoundException: IResourcePack`
- `JadedMaps` → `ClassNotFoundException: CommonProxy`

Attempted workarounds (CF_EXCLUDE_MODS) only fix one mod at a time.

**Real fix:** Use the official Agrarian Skies 2 **server pack** from CurseForge:
- URL: `https://mediafilez.forgecdn.net/files/3016/706/Agrarian%2BSkies%2B2%2B%282.0.6%29-Server.zip`
- Version: 2.0.6 (server pack version, MC 1.7.10, Forge 10.13.4.1614)
- Setup guide: https://legacy.curseforge.com/minecraft/modpacks/agrarian-skies-2/pages/setting-up-an-agrarian-skies-2-server

### 3. itzg Modpack Extraction Failures (CURRENT)
The server pack zip has correct structure (`mods/`, `config/`, `scripts/`, `maps/`
at root level). But itzg mishandles extraction:

- `TYPE=CURSEFORGE` + `CF_SERVER_MOD`: Fails with "Modpack missing start script
  and unable to find Forge jar to generate one" (old pack has no start script)
- `TYPE=FORGE` + `GENERIC_PACK_URL`: Mods dir ends up empty (extraction timing
  issue or overwritten by Forge installer)
- `TYPE=FORGE` + `MODPACK`: Extracts zip INTO `/data/mods/` instead of `/data/`,
  so mods land at `/data/mods/mods/*.jar` — wrong path

**Current fix (in progress):** Download and extract the server pack zip on the
HOST side before starting the Docker container. Let itzg only handle Forge
installation (`TYPE=FORGE` + `FORGE_VERSION=10.13.4.1614`). This gives us full
control over extraction and avoids itzg's broken modpack handling for old packs.

## Key Details
- Minecraft: 1.7.10
- Forge: 10.13.4.1614
- Java: 8 (Docker image: `itzg/minecraft-server:java8`)
- Server pack has ~93 mod jars, configs, scripts, maps, resources
- `ModpackSource::ForgeWithPack` variant handles this pattern
