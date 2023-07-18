DROP TABLE IF EXISTS shifts;
DROP TABLE IF EXISTS volunteers;

CREATE TABLE volunteers (
  card_id smallint NOT NULL PRIMARY KEY,
  phone_number VARCHAR(20) NOT NULL,
  name VARCHAR(60) NOT NULL,
  surname VARCHAR(50) NOT NULL,
  disabled BOOLEAN NOT NULL
);

CREATE TABLE shifts (
  id serial PRIMARY KEY,
  date DATE NOT NULL,
  task smallint NOT NULL,
  card_id smallint NOT NULL,
  FOREIGN KEY(card_id)
      REFERENCES volunteers(card_id)
);
