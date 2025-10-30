cp gdm.json gdm_backup.json
jq 'del(.plugins.gut)' gdm.json > tmp.json && mv tmp.json gdm.json
vhs ./vhs/gdm_add.tape
vhs ./vhs/gdm_install.tape
vhs ./vhs/gdm_intro.tape
vhs ./vhs/gdm_remove.tape
vhs ./vhs/gdm_search.tape

cp gdm_backup.json gdm.json
vhs ./vhs/gdm_outdated.tape
vhs ./vhs/gdm_update.tape
cp gdm_backup.json gdm.json
rm gdm_backup.json