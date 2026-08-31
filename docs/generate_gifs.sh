cp gdm.toml gdm_backup.toml
vhs ./vhs/gdm_add.tape
vhs ./vhs/gdm_add_git.tape
vhs ./vhs/gdm_install.tape
vhs ./vhs/gdm_intro.tape
vhs ./vhs/gdm_remove.tape
vhs ./vhs/gdm_search.tape

cp gdm_backup.toml gdm.toml
vhs ./vhs/gdm_outdated.tape
vhs ./vhs/gdm_update.tape
cp gdm_backup.toml gdm.toml
rm gdm_backup.toml
