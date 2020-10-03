/*
 * The user, subscription, and viewed tables are all that is needed for this application. The episode, program, and log tables
 * are just "caches" of the tvmaze data to void api limits. Program name, episode title, and all URL lengths are
 * "reasonable guesses" as tvmaze doesn't document their types.
 */
ALTER DATABASE program_guide CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- Cleanup some odd data.
DELETE
FROM status
WHERE season = 'P';

DELETE
FROM status
WHERE episode_number = '0';

-- No longer using this table.
DROP table torrent_site;

-- Drop defaults and adjust sizes.
ALTER TABLE user
    MODIFY id INTEGER UNSIGNED AUTO_INCREMENT NOT NULL,
    MODIFY permissions TINYINT UNSIGNED NOT NULL,
    MODIFY last_login_date DATETIME NULL,
    ALTER username DROP DEFAULT,
    ALTER password DROP DEFAULT,
    ALTER registration_date DROP DEFAULT;

-- Adjust sizes and types. Add tvmaze id (will be null for now).
ALTER TABLE episode
    DROP production_code,
    DROP serial_number,
    MODIFY season TINYINT UNSIGNED NOT NULL,
    MODIFY number TINYINT UNSIGNED NOT NULL,
    MODIFY title VARCHAR(128),
    MODIFY summary_url VARCHAR(255),
    ADD tvmaze_show_id INTEGER UNSIGNED FIRST;

-- Populate the new tvmaze id.
UPDATE episode e
    INNER JOIN program p on e.program_id = p.id
SET e.tvmaze_show_id = p.tvmaze_id
WHERE e.tvmaze_show_id IS NULL;

-- Drop old program id and make tvmaze id part of the key.
ALTER TABLE episode
    MODIFY tvmaze_show_id INTEGER UNSIGNED NOT NULL,
    DROP PRIMARY KEY,
    ADD PRIMARY KEY (tvmaze_show_id, season, number),
    DROP program_id;

-- New table to track subscriptions.
CREATE TABLE subscription
(
    user_id        INTEGER UNSIGNED NOT NULL,
    tvmaze_show_id INTEGER UNSIGNED NOT NULL,
    UNIQUE (user_id, tvmaze_show_id)
);

-- Populate new subscription table from old subscribed table.
INSERT INTO subscription (user_id, tvmaze_show_id)
SELECT s.user_id, p.tvmaze_id
FROM subscribed s
         INNER JOIN program p ON s.program_id = p.id;

-- No longer using this table.
DROP TABLE subscribed;

-- Create a new table to track what has been viewed.
CREATE TABLE viewed
(
    user_id        INTEGER UNSIGNED NOT NULL,
    tvmaze_show_id INTEGER UNSIGNED NOT NULL,
    season         TINYINT UNSIGNED NOT NULL,
    number         TINYINT UNSIGNED NOT NULL
);

-- Populate new viewed table based on old status table.
INSERT INTO viewed (user_id, tvmaze_show_id, season, number)
SELECT s.user_id, p.tvmaze_id, s.season, s.episode_number
FROM status s
         INNER JOIN program p ON s.program_id = p.id;

-- No longer using this table.
DROP TABLE status;

-- Modify types and sizes. Drop old primary key.
ALTER TABLE program
    DROP do_update,
    DROP id,
    CHANGE COLUMN tvmaze_id tvmaze_show_id INTEGER UNSIGNED NOT NULL PRIMARY KEY FIRST,
    ADD INDEX (name),
    MODIFY name VARCHAR(128),
    MODIFY url VARCHAR(255),
    MODIFY network VARCHAR(128);

-- Adjust size.
ALTER TABLE log
    MODIFY id INTEGER UNSIGNED NOT NULL;

-- Move from latin to utf8.
ALTER TABLE episode
    CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
ALTER TABLE program
    CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
ALTER TABLE log
    CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- Force episodes to be updated in case there were some utf8 data that got lost.
UPDATE program
SET last_update = NULL;

-- Switch engine type for foreign keys.
ALTER TABLE episode
    ENGINE = InnoDB;
ALTER TABLE log
    ENGINE = InnoDB;
ALTER TABLE program
    ENGINE = InnoDB;
ALTER TABLE subscription
    ENGINE = InnoDB;
ALTER TABLE user
    ENGINE = InnoDB;
ALTER TABLE viewed
    ENGINE = InnoDB;

-- Add foreign key constraints.
ALTER TABLE episode
    ADD FOREIGN KEY (tvmaze_show_id) REFERENCES program (tvmaze_show_id);
ALTER TABLE subscription
    ADD FOREIGN KEY (tvmaze_show_id) REFERENCES program (tvmaze_show_id),
    ADD FOREIGN KEY (user_id) REFERENCES user (id);
ALTER TABLE viewed
    ADD FOREIGN KEY (tvmaze_show_id) REFERENCES program (tvmaze_show_id),
    ADD FOREIGN KEY (user_id) REFERENCES user (id);


-- need to setup own local service to test blanks and nulls