ALTER TABLE recaps
    ADD end_identify_time TIMESTAMP NULL;

ALTER TABLE recap_positions
    ADD identify_time TIMESTAMP NULL;