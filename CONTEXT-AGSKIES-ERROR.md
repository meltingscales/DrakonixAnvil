so, ag skies failed. help me understand why:

[0;39m[39m[mc-image-helper] 21:14:59.401 INFO  : Downloaded mod file mods/ExtraTiC-1.7.10-1.4.6.jar
[0;39m[39m[mc-image-helper] 21:14:59.430 INFO  : Downloaded mod file mods/ThaumicTinkerer-2.5-1.7.10-164.jar
[0;39m[39m[mc-image-helper] 21:14:59.639 INFO  : Downloaded mod file mods/Botania r1.8-249.jar
[0;39m[39m[mc-image-helper] 21:14:59.793 INFO  : Downloaded mod file mods/OpenModsLib-1.7.10-0.10.1.jar
[0;39m[39m[mc-image-helper] 21:15:00.079 INFO  : Downloaded mod file mods/Automagy-1.7.10-0.28.2.jar
[0;39m[39m[mc-image-helper] 21:15:00.102 INFO  : Downloaded mod file mods/Chisel-2.9.5.11.jar
[0;39m[39m[mc-image-helper] 21:15:00.226 INFO  : Downloaded mod file mods/OpenBlocks-1.7.10-1.6.jar
[0;39m[39m[mc-image-helper] 21:15:00.667 INFO  : Downloading Forge installer 10.13.4.1614 for Minecraft 1.7.10
[0;39m[39m[mc-image-helper] 21:15:01.708 INFO  : Running Forge 10.13.4.1614 installer for Minecraft 1.7.10. This might take a while...
[0;39m[init] Copying any mods from /mods to /data/mods
[init] Copying any configs from /config to /data/config
[init] Creating server properties in /data/server.properties
[init] Disabling whitelist functionality
[39m[mc-image-helper] 21:15:14.517 INFO  : Created/updated 4 properties in /data/server.properties
[0;39m[init] Setting initial memory to 4096M and max to 4096M
[init] Starting the Minecraft server...
A problem occurred running the Server launcher.java.lang.reflect.InvocationTargetException
	at java.base/jdk.internal.reflect.DirectMethodHandleAccessor.invoke(Unknown Source)
	at java.base/java.lang.reflect.Method.invoke(Unknown Source)
	at cpw.mods.fml.relauncher.ServerLaunchWrapper.run(ServerLaunchWrapper.java:43)
	at cpw.mods.fml.relauncher.ServerLaunchWrapper.main(ServerLaunchWrapper.java:12)
Caused by: java.lang.ClassCastException: class jdk.internal.loader.ClassLoaders$AppClassLoader cannot be cast to class java.net.URLClassLoader (jdk.internal.loader.ClassLoaders$AppClassLoader and java.net.URLClassLoader are in module java.base of loader 'bootstrap')
	at net.minecraft.launchwrapper.Launch.<init>(Launch.java:34)
	at net.minecraft.launchwrapper.Launch.main(Launch.java:28)
	... 4 more
2026-02-05T21:15:15.276Z	WARN	mc-server-runner	Minecraft server failed. Inspect logs above for errors that indicate cause. DO NOT report this line as an error.	{"exitCode": 1}
2026-02-05T21:15:15.276Z	INFO	mc-server-runner	Done

