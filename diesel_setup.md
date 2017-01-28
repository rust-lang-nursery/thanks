# Diesel setup
These were the tasks I needed to perform before I could get diesel_cli installed.
## Ubunutu
These are the packages referenced in the README that need to be installed for postgresql and sqlite 3:
* `postgresql`
* `postgresql-contrib`
* `libpq-dev` (Not enough to just set up postgresql, need this for `libpq`)
* `sqlite3`

Then needed to set up postgresql using [this](https://help.ubuntu.com/community/PostgreSQL) tutorial.

After this was able to install diesel_cli and then run the setup with the newly created credentials following the instructions in [README](README.md)
