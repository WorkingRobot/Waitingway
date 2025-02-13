CREATE TYPE roulette_role AS ENUM ('tank', 'healer', 'dps');

CREATE TABLE roulette_sizes
(
    datacenter_id       SMALLINT        NOT NULL,
    languages           SMALLINT        NOT NULL,
    roulette_id         SMALLINT        NOT NULL,
    role                roulette_role   NOT NULL,

    size_user_id        UUID,
    size_time           TIMESTAMP,
    size                SMALLINT,

    est_time_user_id   UUID,
    est_time_time      TIMESTAMP,
    est_time           SMALLINT,

    wait_time_user_id   UUID,
    wait_time_time      TIMESTAMP,
    wait_time           DOUBLE PRECISION,
    
    PRIMARY KEY (datacenter_id, languages, roulette_id, role)
);