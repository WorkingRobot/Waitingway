CREATE TABLE IF NOT EXISTS travel_states
(
    world_id SMALLINT    NOT NULL,
    time     TIMESTAMP   NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC'),
    travel   BOOLEAN NOT NULL,
    accept   BOOLEAN NOT NULL,
    prohibit BOOLEAN NOT NULL,
    
    PRIMARY KEY (world_id, time)
);

CREATE TABLE IF NOT EXISTS travel_times
(
    time        TIMESTAMP   PRIMARY KEY DEFAULT (NOW() AT TIME ZONE 'UTC'),
    travel_time INT         NOT NULL
);

CREATE INDEX IF NOT EXISTS travel_states_time_idx ON travel_states(time);