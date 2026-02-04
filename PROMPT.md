# prompt
- make sure to view "/home/henrypost/Git/VirtualMachineConfigs/ansible/minecraft" for inspiration.
- read README.md

- let's make sure "Agrarian Skies 2" from FTB works as a test pack. it has a special place in my heart.

## ideas
- hook into `stdin` for java process, expose it to the rust frontend
  - perhaps use `tee` or `screen` for this? how does enterprise do it?
  - or, just use RCON.
- support backup to google drive, opinionated `~/DrakonixAnvilMinecraftBackup/**`

- how to help n00bs port forward?
  - remote `nc` to tell you if tcp/12345 is exposed? jeb util?

- CICD - automated binary releases for windows, macos, and linux. only build on 'v1.0' 'v2.0' etc tagged releases. when tags get pushed.
