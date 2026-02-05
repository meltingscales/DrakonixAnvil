# prompt
- make sure to view "/home/henrypost/Git/VirtualMachineConfigs/ansible/minecraft" for inspiration.
- read README.md

- let's make sure "Agrarian Skies 2" from FTB works as a test pack. it has a special place in my heart.

## ideas
- ✅ ~~hook into `stdin` for java process~~ - used RCON instead (custom implementation in `src/rcon.rs`)
- support backup to google drive, opinionated `~/DrakonixAnvilMinecraftBackup/**`

- how to help n00bs port forward?
  - remote `nc` to tell you if tcp/12345 is exposed? jeb util?

- ✅ CICD - automated binary releases for windows, macos, and linux. only build on 'v1.0' 'v2.0' etc tagged releases. when tags get pushed.

## workflow
- use **feature branches** for development
- PR to `main` triggers CI (check, clippy, fmt, build)
- merge to `main` runs CI again
- to release: `git tag v0.2.0 && git push origin v0.2.0` triggers Release workflow
