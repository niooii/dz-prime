CREATE TABLE IF NOT EXISTS settings (
    user_id     TEXT PRIMARY KEY NOT NULL,
    ack_phrase  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id              BIGSERIAL PRIMARY KEY NOT NULL,
    user_id         TEXT NOT NULL,
    title           TEXT NOT NULL,
    info            TEXT NOT NULL,
    time_created    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    remind_at       TIME NOT NULL,
    -- either (on_days and repeat_weekly) or date must be non null
    on_date         DATE,
    -- 1-indexed starting from sunday: 1-sunday, 2-monday, etc..
    on_days         INT[],
    repeat_weekly   BOOLEAN NOT NULL DEFAULT false
);