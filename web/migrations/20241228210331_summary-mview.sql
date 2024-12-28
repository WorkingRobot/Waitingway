CREATE MATERIALIZED VIEW IF NOT EXISTS world_summary AS
    SELECT
    w.world_id,
    w.world_name,
    w.datacenter_id,
    w.datacenter_name,
    w.region_id,
    w.region_abbreviation,
    w.region_name,
    ws.status,
    ws.category,
    ws.can_create,
    ts.prohibit,
    qe.time,
    qe.size,
    qe.duration
    FROM
    worlds w
    INNER JOIN (
        SELECT
        DISTINCT ON (world_id) world_id,
        prohibit
        FROM
        travel_states
        ORDER BY
        world_id,
        time DESC
    ) ts ON w.world_id = ts.world_id
    INNER JOIN (
        SELECT
        DISTINCT ON (world_id) world_id,
        status,
        category,
        can_create
        FROM
        world_statuses
        ORDER BY
        world_id,
        time DESC
    ) ws ON w.world_id = ws.world_id
    INNER JOIN (
        SELECT
        *
        FROM
        queue_estimates
    ) qe ON w.world_id = qe.world_id
    WHERE
    w.hidden = FALSE;
    
CREATE UNIQUE INDEX ON world_summary(world_id);