# Run as a non-root user

The image may be run with a numeric user when the mounted directories are writable by that user:

```bash
docker run -d \
  --name rclone-backup \
  --user 1100:1100 \
  -p 127.0.0.1:8080:8080 \
  -v /srv/rclone-backup/config:/config \
  -v /srv/data:/data:ro \
  czyt/rclone-backup:2.0.7
```

Before starting, create `/srv/rclone-backup/config/rclone` and make `/srv/rclone-backup/config` writable by `1100:1100`. The service needs to create SQLite, its encryption key, the rclone config, and temporary work files. Source directories only need read and traversal permission.
