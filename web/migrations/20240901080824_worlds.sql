CREATE TABLE IF NOT EXISTS worlds
(
    world_id            SMALLINT    PRIMARY KEY,
    world_name          VARCHAR     NOT NULL,
    datacenter_id       SMALLINT    NOT NULL,
    datacenter_name     VARCHAR     NOT NULL,
    region_id           SMALLINT    NOT NULL,
    region_name         VARCHAR     NOT NULL,
    is_cloud            BOOLEAN     NOT NULL
);