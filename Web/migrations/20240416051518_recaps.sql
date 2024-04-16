CREATE TABLE IF NOT EXISTS recaps
(
    id              UUID        PRIMARY KEY,
    user_id         UUID        NOT NULL,
    world_id        SMALLINT    NOT NULL,
    successful      BOOLEAN     NOT NULL,
    start_time      TIMESTAMP   NOT NULL,
    end_time        TIMESTAMP   NOT NULL
);

CREATE TABLE IF NOT EXISTS recap_positions
(
    recap_id    UUID        NOT NULL REFERENCES recaps ON DELETE CASCADE,
    time        TIMESTAMP   NOT NULL,
    position    INT         NOT NULL,
    
    PRIMARY KEY (recap_id, time)
);