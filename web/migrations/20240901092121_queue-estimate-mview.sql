CREATE MATERIALIZED VIEW IF NOT EXISTS queue_estimates AS
    SELECT
        recaps.world_id as world_id,
        cast(COALESCE(EXTRACT(EPOCH FROM (recaps.end_time - recap_position.time)), 0) as double precision) as duration,
        queue_size.size as size,
        queue_size.time as time
    FROM
        recaps
        INNER JOIN LATERAL (
            SELECT min(time) as time
            FROM recap_positions
            WHERE recap_positions.recap_id = recaps.id
        ) recap_position ON TRUE
        INNER JOIN LATERAL (
            SELECT size, time
            FROM queue_sizes
            WHERE queue_sizes.world_id = recaps.world_id
        ) queue_size ON TRUE
    WHERE
        id IN (
            SELECT id
            FROM recaps
                INNER JOIN (
                    SELECT
                        world_id,
                        max(start_time) AS start_time
                    FROM recaps
                    GROUP BY world_id
                ) AS sub ON recaps.world_id = sub.world_id
                AND recaps.start_time = sub.start_time
            )
    ORDER BY world_id;