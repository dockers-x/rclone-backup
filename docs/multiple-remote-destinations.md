# Multiple remote destinations

Add each rclone account in **Storage accounts**, then add one or more destinations to a backup plan. The editor displays configured aliases from rclone and does not accept unknown free-form aliases.

For v1 migration, consecutively numbered variables remain supported:

```text
RCLONE_REMOTE_NAME_1=Primary
RCLONE_REMOTE_DIR_1=/RcloneBackup/
RCLONE_REMOTE_NAME_2=Offsite
RCLONE_REMOTE_DIR_2=/RcloneBackup/
```

These values are imported once. Later changes belong in the Web UI/API.
