-- CREATE TYPE content_flags AS (
--     loot_rule           loot_rule   NOT NULL,
--     is_unrestricted     BOOLEAN     NOT NULL,
--     is_min_ilvl         BOOLEAN     NOT NULL,
--     is_silence_echo     BOOLEAN     NOT NULL,
--     is_explorer         BOOLEAN     NOT NULL,
--     is_level_synced     BOOLEAN     NOT NULL,
--     is_limited_leveling BOOLEAN     NOT NULL,
--     in_progress_party   BOOLEAN     NOT NULL
-- );

CREATE TYPE duty_update_type AS ENUM ('none', 'roulette', 'thd', 'players', 'wait_time');

CREATE TYPE duty_party_member AS (
    job     SMALLINT,
    level   SMALLINT,
    world   SMALLINT
);

CREATE TYPE duty_fill_param AS (
    found   SMALLINT,
    needed  SMALLINT
);
