ALTER TABLE worlds RENAME region_name TO region_abbreviation;
ALTER TABLE worlds ADD region_name VARCHAR NOT NULL DEFAULT '';