# prompt
- make sure to view "/home/henrypost/Git/VirtualMachineConfigs/ansible/minecraft" for inspiration.
- read README.md, we will want to make sure our features actually work.
- read CONTEXT.md, you can use it to save your work.

- read CONTEXT*.md for other WIP projects.

- see ./example-ansible-seaopolis-submerged/** for examples of old ansible code that inspired this project.

- let's make sure "Agrarian Skies 2" from FTB works as a test pack. it has a special place in my heart.

## ideas
- support backup to google drive, opinionated `~/DrakonixAnvilMinecraftBackup/**`

- how to help n00bs port forward?
  - remote `nc` to tell you if tcp/12345 is exposed? jeb util?

## workflow
- use **feature branches** for development
- PR to `main` triggers CI (check, clippy, fmt, build)
- merge to `main` runs CI again
- to release: `git tag v0.2.0 && git push origin v0.2.0` triggers Release workflow
