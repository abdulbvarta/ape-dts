INSERT INTO test_db_1.one_pk_no_uk_1 VALUES(1, 1),(2, 2);
INSERT INTO test_db_1.one_pk_no_uk_2 VALUES(1, 1),(2, 2);

UPDATE test_db_1.one_pk_no_uk_1 SET f_1 = 3;
UPDATE test_db_1.one_pk_no_uk_2 SET f_1 = 3;

DELETE FROM test_db_1.one_pk_no_uk_1;
DELETE FROM test_db_1.one_pk_no_uk_2;

INSERT INTO test_db_2.one_pk_no_uk_1 VALUES(1, 1),(2, 2);
INSERT INTO test_db_2.one_pk_no_uk_2 VALUES(1, 1),(2, 2);

UPDATE test_db_2.one_pk_no_uk_1 SET f_1 = 3;
UPDATE test_db_2.one_pk_no_uk_2 SET f_1 = 3;

DELETE FROM test_db_2.one_pk_no_uk_1;
DELETE FROM test_db_2.one_pk_no_uk_2;

INSERT INTO test_db_3.one_pk_no_uk_1 VALUES(1, 1),(2, 2);
INSERT INTO test_db_3.one_pk_no_uk_2 VALUES(1, 1),(2, 2);

UPDATE test_db_3.one_pk_no_uk_1 SET f_1 = 3;
UPDATE test_db_3.one_pk_no_uk_2 SET f_1 = 3;

DELETE FROM test_db_3.one_pk_no_uk_1;
DELETE FROM test_db_3.one_pk_no_uk_2;

