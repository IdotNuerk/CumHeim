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