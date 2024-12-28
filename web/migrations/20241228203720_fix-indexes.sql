DROP INDEX travel_states_time_idx;
ALTER TABLE travel_states DROP CONSTRAINT travel_states_pkey;
CREATE UNIQUE INDEX travel_states_pkey ON travel_states(world_id, time DESC);

DROP INDEX world_statuses_time_idx;
ALTER TABLE world_statuses DROP CONSTRAINT world_statuses_pkey;
CREATE UNIQUE INDEX world_statuses_pkey ON world_statuses(world_id, time DESC);