DROP TABLE IF EXISTS shifts;
DROP TABLE IF EXISTS volunteers;

CREATE TABLE volunteers (
  card_id INTEGER NOT NULL PRIMARY KEY,
  fiscal_code VARCHAR(20) NOT NULL,
  name VARCHAR(60) NOT NULL,
  surname VARCHAR(50) NOT NULL,
  disabled BOOLEAN NOT NULL
);

CREATE TABLE shifts (
  id serial PRIMARY KEY,
  date VARCHAR(10) NOT NULL,
  day VARCHAR(9) NOT NULL,
  task VARCHAR(40) NOT NULL,
  entrance_hour VARCHAR(5) NOT NULL,
  exit_hour VARCHAR(5) NOT NULL,
  row_deadline TIMESTAMP,
  card_id INTEGER NOT NULL,
  FOREIGN KEY(card_id)
      REFERENCES volunteers(card_id)
);
