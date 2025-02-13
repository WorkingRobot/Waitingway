CREATE TYPE job_disciple AS ENUM ('war', 'magic', 'hand', 'land');

CREATE TABLE jobs
(
    id                  SMALLINT        PRIMARY KEY,
    name                VARCHAR         NOT NULL,
    abbreviation        VARCHAR         NOT NULL,
    disciple            job_disciple    NOT NULL,
    role                roulette_role,
    can_queue           BOOLEAN         NOT NULL
);