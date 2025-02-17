DELETE FROM travel_states 
WHERE ctid IN (
    SELECT ctid
    FROM (
        SELECT 
            ctid,
            world_id,
            travel,
            accept,
            prohibit,
            LAG(travel) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_travel,
            LAG(accept) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_accept,
            LAG(prohibit) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_prohibit
        FROM travel_states
    ) sub
    WHERE travel = prev_travel
      AND accept = prev_accept
      AND prohibit = prev_prohibit
);

DELETE FROM world_statuses 
WHERE ctid IN (
    SELECT ctid
    FROM (
        SELECT 
            ctid,
            world_id,
            status,
            category,
            can_create,
            LAG(status) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_status,
            LAG(category) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_category,
            LAG(can_create) OVER (PARTITION BY world_id ORDER BY time DESC) AS prev_can_create
        FROM world_statuses
    ) sub
    WHERE status = prev_status
      AND category = prev_category
      AND can_create = prev_can_create
);