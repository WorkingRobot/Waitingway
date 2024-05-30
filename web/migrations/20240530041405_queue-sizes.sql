CREATE TABLE IF NOT EXISTS queue_sizes
(
    user_id         UUID        NOT NULL,
    world_id        SMALLINT    NOT NULL,
    time            TIMESTAMP   NOT NULL,
    size            INT         NOT NULL
);