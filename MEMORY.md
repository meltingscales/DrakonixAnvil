# DrakonixAnvil - Project Memory

## itzg/minecraft-server Lessons

### Java Version Selection
- Image tags: `:java8`, `:java11`, `:java17`, `:java21`, `:latest`
- MC 1.7.10 + Forge requires Java 8 (URLClassLoader removed in Java 9+)
- Per-server `java_version` field maps to Docker image tag via `docker_image()`

### What DOESN'T Work for Old Packs (1.7.10 Ag Skies 2)
1. `AUTO_CURSEFORGE` — downloads CLIENT manifest, includes client-only mods that crash server
2. `CF_EXCLUDE_MODS` — only works for CF-hosted mods, whack-a-mole approach
3. `TYPE=CURSEFORGE` + `CF_SERVER_MOD` — fails with "missing start script" on old packs
4. `TYPE=FORGE` + `GENERIC_PACK_URL` — mods dir ends up empty (timing issue)
5. `TYPE=FORGE` + `MODPACK` — extracts zip INTO /data/mods/ instead of /data/

### Solution: ForgeWithPack (host-side extraction)
- `src/pack_installer.rs` downloads server pack zip via reqwest on host
- Extracts to server data directory (bind-mounted as /data/)
- Starts container with just `TYPE=FORGE` + `FORGE_VERSION`
- itzg installs Forge, finds mods already in place
- `.pack_installed` marker prevents re-download

### Skyblock Packs
- Need `LEVEL=maps/<map name>` in extra_env to use included skyblock map
- Without it, server generates a normal world (void for skyblock = instant death)
- Ag Skies 2 uses `LEVEL=maps/Default Platform - Normal`

## Conventions
- `#![deny(warnings)]` enforced — must fix all warnings before committing
- Run `cargo clippy` and `cargo build` before committing
- serde aliases for backward compat when renaming enum variants (e.g. `FTB` -> `Ftb`)
- Commits on main, `Co-Authored-By` trailer for Claude
