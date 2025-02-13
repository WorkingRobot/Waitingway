CREATE TABLE IF NOT EXISTS duty_recaps
(
    id                  UUID        PRIMARY KEY,
    user_id             UUID        NOT NULL,

    queued_roulette     SMALLINT,
    queued_content      SMALLINT[],
    queued_job          SMALLINT    NOT NULL,
    queued_flags        SMALLINT    NOT NULL,

    world_id            SMALLINT    NOT NULL,

    is_party_leader     BOOLEAN     NOT NULL,
    party_members       duty_party_member[],
    start_time          TIMESTAMP   NOT NULL,
    end_time            TIMESTAMP   NOT NULL,
    withdraw_message    SMALLINT,

    client_version      VARCHAR     NULL
);

CREATE TABLE IF NOT EXISTS duty_updates
(
    recap_id    UUID                NOT NULL REFERENCES duty_recaps ON DELETE CASCADE,
    time        TIMESTAMP           NOT NULL,
    reserving   BOOLEAN             NOT NULL,

    update_type duty_update_type    NOT NULL,

    wait_time   SMALLINT,
    position    SMALLINT,
    fill_params duty_fill_param[],
    
    PRIMARY KEY (recap_id, time)
);

CREATE TABLE IF NOT EXISTS duty_pops
(
    recap_id            UUID        NOT NULL REFERENCES duty_recaps ON DELETE CASCADE,
    time                TIMESTAMP   NOT NULL,
    flags               SMALLINT    NOT NULL,
    content             SMALLINT,
    in_progress_time    TIMESTAMP,
    
    PRIMARY KEY (recap_id, time)
);