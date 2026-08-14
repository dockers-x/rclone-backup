FROM rust:1.97-alpine AS builder

WORKDIR /src
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src
COPY web ./web
RUN cargo build --locked --release -p rclone-backup

FROM rclone/rclone:1.75.0

ENV RCLONE_CONFIG=/config/rclone/rclone.conf \
    RCLONE_BACKUP_ADDR=0.0.0.0:8080 \
    RCLONE_BACKUP_DATABASE_URL=sqlite:///config/rclone-backup.db?mode=rwc \
    RCLONE_BACKUP_WORK_DIR=/tmp/rclone-backup

RUN apk add --no-cache 7zip ca-certificates curl s-nail tzdata \
    && mkdir -p /config/rclone /tmp/rclone-backup

COPY --from=builder /src/target/release/rclone-backup /usr/local/bin/rclone-backup
COPY LICENSE THIRD_PARTY_NOTICES.md /usr/share/licenses/rclone-backup/

EXPOSE 8080
VOLUME ["/config"]
ENTRYPOINT ["/usr/local/bin/rclone-backup"]
