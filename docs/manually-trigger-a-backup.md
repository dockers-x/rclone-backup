# Manually trigger a backup

Use **Run now** in the Web UI for the normal path. The service returns a run ID immediately and records progress in Run history.

The compatible command-line path remains available:

```bash
docker exec rclone-backup rclone-backup backup PLAN_ID
```

When there is exactly one plan, `PLAN_ID` can be omitted. The command waits for completion and exits. If rclone has no configured remote, it returns `RCLONE_NOT_READY`; the normal service process remains online.
