-- DATA SUBJECT TABLE.
CREATE DATA_SUBJECT TABLE users (
    email varchar(255),
    apikey varchar(255),
    is_admin int,
    consent int,
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
    FOREIGN KEY (email) OWNED_BY users(email),
    FOREIGN KEY (lec) REFERENCES lectures(id),
    FOREIGN KEY (question_id) REFERENCES questions(id)
);

-- A presenter owns the record that marks them as a presenter of some lecture.
CREATE TABLE presenters (
    id int AUTO_INCREMENT,
    lecture_id int,
    email varchar(255) OWNED_BY users(email),
    PRIMARY KEY (id),
    FOREIGN KEY (lecture_id) REFERENCES lectures(id)
);

CREATE VIEW lectures_with_question_counts AS '"
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
ORDER BY id
"';

-- Answers from consenting students only, paired with the lecture/question
-- they belong to. Feeds the admin automated-analysis endpoint: the `answer`
-- column carries AutomatedAnalysisPolicy (see
-- policies/automated_analysis.rs), not AnswerAccessPolicy, which re-checks
-- consent at declassification time rather than trusting this filter alone.
CREATE VIEW consented_answers AS '"
SELECT answers.answer AS answer, users.consent AS consent, answers.lec AS lec, answers.question_id AS question_id
FROM answers JOIN users ON answers.email = users.email
WHERE users.consent = 1 AND lec = ? AND question_id = ?
"';
