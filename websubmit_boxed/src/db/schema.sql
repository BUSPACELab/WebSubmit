CREATE TABLE users (
    email varchar(255),
    apikey varchar(255),
    is_admin tinyint,
    consent tinyint,
    PRIMARY KEY (email),
    UNIQUE (apikey)
);

-- lectures and questions table are unsharded.
CREATE TABLE lectures (
    id int,
    label varchar(255),
    PRIMARY KEY (id)
);
-- question_number: number *within* lecture
CREATE TABLE questions (
    id int AUTO_INCREMENT,
    lecture_id int,
    question_number int,
    question text,
    PRIMARY KEY (id),
    FOREIGN KEY (lecture_id) REFERENCES lectures(id)
);

-- Answers are owned by the student that provided the answer.
-- id = format!('{}-{}', email, question_id)
-- lec is denormalised from questions so that policies can resolve the lecture
-- without a join.
CREATE TABLE answers (
    id varchar(255),
    email varchar(255),
    lec int,
    question_id int,
    answer text,
    submitted_at datetime,
    PRIMARY KEY (id),
    FOREIGN KEY (email) REFERENCES users(email),
    FOREIGN KEY (lec) REFERENCES lectures(id),
    FOREIGN KEY (question_id) REFERENCES questions(id)
);

-- A presenter owns the record that marks them as a presenter of some lecture.
CREATE TABLE presenters (
    id int AUTO_INCREMENT,
    lecture_id int,
    email varchar(255),
    PRIMARY KEY (id),
    FOREIGN KEY (lecture_id) REFERENCES lectures(id),
    FOREIGN KEY (email) REFERENCES users(email)
);

CREATE VIEW lectures_with_question_counts AS
(
    SELECT lectures.id AS id, lectures.label, 0 AS U_c
    FROM lectures LEFT JOIN questions ON (lectures.id = questions.lecture_id)
    WHERE questions.id IS NULL
    GROUP BY lectures.id, lectures.label
)
UNION
(
    SELECT lectures.id AS id, lectures.label, COUNT(*) AS U_c
    FROM lectures JOIN questions ON (lectures.id = questions.lecture_id)
    GROUP BY lectures.id, lectures.label
)
ORDER BY id;

-- Every (question, user) pair, with that user's answer when they have given one.
-- Questions are paired with users up front so that the LEFT JOIN keeps questions
-- a user has not answered; filtering `answers.email` in a WHERE clause instead
-- would drop questions that only *other* users have answered.
CREATE VIEW questions_with_answers AS
SELECT
    questions.id AS id,
    questions.lecture_id AS lecture_id,
    questions.question_number AS question_number,
    questions.question AS question,
    users.email AS user_email,
    answers.id AS answer_id,
    answers.email AS answer_email,
    answers.lec AS lec,
    answers.question_id AS question_id,
    answers.answer AS answer,
    answers.submitted_at AS submitted_at
FROM questions
CROSS JOIN users
LEFT JOIN answers
    ON (answers.question_id = questions.id AND answers.email = users.email);
