DROP MATERIALIZED VIEW world_summary;
DROP MATERIALIZED VIEW queue_estimates;

--

DROP INDEX recaps_start_time_index;
CREATE INDEX recaps_world_time_index ON recaps (world_id, start_time DESC) WHERE successful AND NOT reentered;

CREATE MATERIALIZED VIEW queue_estimates AS
    SELECT 
        w.world_id as world_id,
        cast(COALESCE(EXTRACT(EPOCH FROM (r.end_time - p.time)), 0) as double precision) as duration,
        q.size as size,
        q.time as time
    FROM worlds w
    CROSS JOIN LATERAL (
        SELECT id, end_time
        FROM recaps r
        WHERE r.world_id = w.world_id
        AND r.successful
        AND NOT r.reentered
        ORDER BY r.start_time DESC
        LIMIT 1
    ) r
    CROSS JOIN LATERAL (
        SELECT min(time) as time
        FROM recap_positions p
        WHERE p.recap_id = r.id
    ) p
    CROSS JOIN LATERAL (
        SELECT size, time
        FROM queue_sizes q
        WHERE q.world_id = w.world_id
    ) q
    ORDER BY w.world_id;

CREATE UNIQUE INDEX ON queue_estimates(world_id);

--

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