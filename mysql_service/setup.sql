CREATE DATABASE restaurant_api;
USE restaurant_api;

CREATE TABLE item_t (
    status ENUM('cooking', 'cooked', 'canceled'),
    table_no SMALLINT UNSIGNED,
    item_no BIGINT UNSIGNED,
    content VARCHAR(20),
    create_at BIGINT UNSIGNED,
    time_take TINYINT UNSIGNED,
    PRIMARY KEY(status, table_no, item_no),
    index(item_no)
);
