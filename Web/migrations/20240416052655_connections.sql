CREATE TABLE IF NOT EXISTS connections
(
    user_id         UUID        NOT NULL,
    created_at      TIMESTAMP   NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC'),

    conn_user_id    BIGINT      NOT NULL,
    username        VARCHAR     NOT NULL,
    display_name    VARCHAR     NOT NULL,
    
    PRIMARY KEY (user_id, conn_user_id)
);