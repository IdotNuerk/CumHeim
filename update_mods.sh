cd /home/steam/valheim/BepInEx/plugins
rm -r *

# Jotunn
wget https://thunderstore.io/package/download/ValheimModding/Jotunn/2.20.0/ -O jot.zip
7z e jot.zip -oJotunn
rm jot.zip

# MultiUserChest
wget https://thunderstore.io/package/download/MSchmoecker/MultiUserChest/0.5.10/ -O muc.zip
7z e muc.zip -oMultiUserChest
rm muc.zip

# ServerSideMap
wget https://github.com/Mydayyy/Valheim-ServerSideMap/releases/download/v1.3.11/ServerSideMap.zip -O ssm.zip
7z e ssm.zip
rm ssm.zip

# PlantEverything
wget https://thunderstore.io/package/download/Advize/PlantEverything/1.17.1/ -O pe.zip
7z e pe.zip
rm pe.zip

# AzuExtendedPlayerInventory
wget https://thunderstore.io/package/download/Azumatt/AzuExtendedPlayerInventory/1.4.0/ -O aepi.zip
7z e aepi.zip
rm aepi.zip

# TargetPortal
wget https://thunderstore.io/package/download/Smoothbrain/TargetPortal/1.1.19/ -O tp.zip
7z e tp.zip
rm tp.zip

# PlantEasily
wget https://thunderstore.io/package/download/Advize/PlantEasily/1.8.0/ -O pez.zip
7z e pez.zip
rm pez.zip

# ServerSyncFix
wget https://thunderstore.io/package/download/JereKuusela/Server_Sync_Fix/1.3.0/ -O ssf.zip
7z e ssf.zip
rm ssf.zip

# NoSmokeStayLit
wget https://thunderstore.io/package/download/TastyChickenLegs/NoSmokeStayLit/2.3.3/ -O nssl.zip
7z e nssl.zip
rm nssl.zip
