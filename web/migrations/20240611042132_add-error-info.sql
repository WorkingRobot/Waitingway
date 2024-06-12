ALTER TABLE recaps
    ADD error_type INTEGER NULL,
    ADD error_code INTEGER NULL,
    ADD error_info VARCHAR NULL,
    ADD error_row SMALLINT NULL;