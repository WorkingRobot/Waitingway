CREATE TABLE IF NOT EXISTS world_statuses
(
    world_id   SMALLINT    NOT NULL,
    time       TIMESTAMP   NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC'),
    status     SMALLINT NOT NULL,
    category   SMALLINT NOT NULL,
    can_create BOOLEAN NOT NULL,
    
    PRIMARY KEY (world_id, time)
);

CREATE INDEX IF NOT EXISTS world_statuses_time_idx ON world_statuses(time);