//! One struct per database table (and the lecture-counts view), named after
//! it, holding exactly that relation's columns with the policy each column
//! carries. Built from a PConRow.
#![allow(dead_code)]
#![allow(non_snake_case)]

mod answers;
mod lectures;
mod lectures_with_question_counts;
mod presenters;
mod questions;
mod users;

pub use answers::AnswerModel;
pub use lectures::LectureModel;
pub use lectures_with_question_counts::LectureWithQuestionCountsModel;
pub use presenters::PresenterModel;
pub use questions::QuestionModel;
pub use users::UserModel;
