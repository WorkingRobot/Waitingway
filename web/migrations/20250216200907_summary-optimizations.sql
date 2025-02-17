DROP MATERIALIZED VIEW world_summary;

CREATE MATERIALIZED VIEW world_summary AS
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
    CROSS JOIN LATERAL (
        SELECT prohibit
        FROM travel_states t
        WHERE t.world_id = w.world_id
        ORDER BY t.time DESC
        LIMIT 1
    ) ts
    CROSS JOIN LATERAL (
        SELECT status, category, can_create
        FROM world_statuses t
        WHERE t.world_id = w.world_id
        ORDER BY t.time DESC
        LIMIT 1
    ) ws
    INNER JOIN (
        SELECT
        *
        FROM
        queue_estimates
    ) qe ON w.world_id = qe.world_id
    WHERE
    w.hidden = FALSE;
    
CREATE UNIQUE INDEX ON world_summary(world_id);