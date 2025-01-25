CREATE TABLE IF NOT EXISTS settings (
    user_id     TEXT PRIMARY KEY NOT NULL,
    ack_phrase  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id          BIGSERIAL PRIMARY KEY NOT NULL,
    user_id     TEXT NOT NULL,
    title       TEXT NOT NULL,
    info        TEXT NOT NULL,
    -- the minute of the day
    remind_at   INT NOT NULL CHECK (remind_at >= 0 AND remind_at < 1440),
    -- either (on_days and repeat_weekly) or date must be non null
    on_date     DATE,
    -- where 0 = sunday, 1 = monday, etc.
    on_days         INTEGER[],
    repeat_weekly   BOOLEAN NOT NULL DEFAULT false
);