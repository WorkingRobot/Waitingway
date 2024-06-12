CREATE TABLE queue_sizes
(
    user_id         UUID        NOT NULL,
    world_id        SMALLINT    PRIMARY KEY,
    time            TIMESTAMP   NOT NULL,
    size            INT         NOT NULL
);