use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{get, post, FromPConForm, PConForm, PConTemplate};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::guards::admin::Admin;
use crate::models::ConsentedAnswerModelOut;
use crate::policies::{AnswerAccessPolicy, ContextData, AUTOMATED_ANALYSIS_REASON};

/// Sent to Claude alongside the lecture material and the anonymized,
/// consenting-student answers for a single `<lec>/<question_id>`.
const ANALYSIS_PROMPT: &str = r#"You are helping a course instructor analyze student responses to a single discussion
question from a class meeting, so they can prepare for the next lecture and identify
where the class is confused or engaged.

## Lecture material
Title: {LECTURE_TITLE}

{PAPER_TEXT}

## Question asked to students
{QUESTION_PROMPT}

## Student answers
Each answer is from a different, anonymized student. Do not try to guess who wrote
what, and do not comment on writing quality, grammar, or English proficiency.

{ANSWERS_LIST}

## Your task
Using the lecture material above as the source of truth, analyze the answers and
produce a report with these sections, in this order: Common themes, Misconceptions
or gaps, Notable answers, Suggested follow-ups.

- Common themes: the 3-6 most recurring ideas or talking points across answers.
  For each, note roughly how many students touched on it (e.g. "~8/20") and
  include 1-2 short verbatim quotes as evidence.

- Misconceptions or gaps: places where multiple students' answers disagree with,
  oversimplify, or miss something in the lecture material. Be specific about what
  the material actually says vs. what students wrote. If you're not confident
  something is actually wrong (vs. just a different valid framing), say so rather
  than asserting it.

- Notable answers: 1-3 answers that show unusually deep engagement, an
  interesting question, or a perspective not covered by the material. Refer to
  them only by their anonymized label (e.g. "Student 7").

- Suggested follow-ups: 2-4 concrete things the instructor could address or
  clarify in the next class based on the above.

If there are too few answers (fewer than ~5) to draw reliable conclusions about
"common" themes, say so explicitly instead of overgeneralizing from a couple of
responses. Ground every claim in the actual answer text -- quote or closely
paraphrase rather than inventing sentiment. If an answer is blank or off-topic,
ignore it rather than commenting on the student.

## Output format
Respond with a single HTML fragment only -- no markdown, no code fences, no
<html>/<head>/<body> wrapper, and no commentary before or after it. Use <h3> for
each of the four section headings, <p> for prose, <ul>/<li> for the theme and
follow-up lists, and <strong> or <em> for emphasis. This fragment gets inserted
directly into an existing admin page, so it must be valid, self-contained HTML."#;

#[derive(Debug, FromPConForm)]
pub(crate) struct AnalysisForm {
    paper_url: PCon<String, NoPolicy>,
}

#[derive(PConRender)]
pub(crate) struct AnalysisRender {
    pub lec_id: PCon<u64, NoPolicy>,
    pub question_id: PCon<u64, NoPolicy>,
    pub result: Option<PCon<String, AnswerAccessPolicy>>,
}

/// The setup form, with no analysis run yet. The form on this page submits
/// to `analysis_submit` below, at the same URL.
#[get("/<lec>/<question_id>")]
pub(crate) fn analysis(
    _admin: Admin,
    lec: PCon<u8, NoPolicy>,
    question_id: PCon<u64, NoPolicy>,
    context: Context<ContextData>,
) -> PConTemplate {
    let ctx = AnalysisRender {
        lec_id: lec.into_pcon::<u64, NoPolicy>(),
        question_id,
        result: None,
    };
    PConTemplate::render("admin/analysis", &ctx, context).unwrap()
}

#[post("/<lec>/<question_id>", data = "<data>")]
pub(crate) fn analysis_submit(
    _admin: Admin,
    lec: PCon<u8, NoPolicy>,
    question_id: PCon<u64, NoPolicy>,
    data: PConForm<AnalysisForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> PConTemplate {
    let paper_url = data.paper_url.clone().discard_box();
    let anthropic_api_key = config.anthropic_api_key.clone();

    let mut bg = backend.lock().unwrap();
    let question_text = bg
        .query_questions("id", (question_id.clone(),), context.clone())
        .into_iter()
        .next()
        .map(|q| q.question.discard_box())
        .unwrap_or_default();

    let lec_num = lec.clone().into_pcon::<u64, NoPolicy>().discard_box();
    let answers = bg.query_consented_answers(
        lec_num,
        question_id.clone().discard_box(),
        context.clone(),
    );
    drop(bg);

    let result = execute_critical(
        answers,
        context.clone(),
        CriticalRegion::new(
            move |answers: Vec<ConsentedAnswerModelOut>, _reason: String| {
                let answers_list = answers
                    .iter()
                    .enumerate()
                    .map(|(i, a)| format!("Student {}: \"{}\"", i + 1, a.answer))
                    .collect::<Vec<_>>()
                    .join("\n");

                let prompt = ANALYSIS_PROMPT
                    .replace("{LECTURE_TITLE}", &format!("Lecture {}", lec_num))
                    .replace("{PAPER_TEXT}", "The paper is attached as a PDF document.")
                    .replace("{QUESTION_PROMPT}", &question_text)
                    .replace("{ANSWERS_LIST}", &answers_list);

                // ureq's "native-tls" feature only makes that connector available;
                // the bare ureq::post() helper only auto-configures TLS when the
                // (heavier, edition2024-requiring) "tls"/rustls feature is on. So
                // build an Agent with the native-tls connector explicitly.
                let tls_connector =
                    native_tls::TlsConnector::new().expect("failed to build TLS connector");
                let agent = ureq::AgentBuilder::new()
                    .tls_connector(std::sync::Arc::new(tls_connector))
                    .build();

                // No streaming: a single request/response round trip.
                let response = agent
                    .post("https://api.anthropic.com/v1/messages")
                    .set("x-api-key", &anthropic_api_key)
                    .set("anthropic-version", "2023-06-01")
                    .set("content-type", "application/json")
                    .send_json(serde_json::json!({
                        "model": "claude-sonnet-5",
                        "max_tokens": 4096,
                        "messages": [{
                            "role": "user",
                            "content": [
                                {
                                    "type": "document",
                                    "source": { "type": "url", "url": paper_url },
                                },
                                { "type": "text", "text": prompt },
                            ],
                        }],
                    }))
                    .expect("Anthropic API request failed");

                let response: serde_json::Value = response
                    .into_json()
                    .expect("failed to parse Anthropic response");
                let text = response["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter_map(|block| block["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();

                // No owner, no lec_id: only the admins-check branch of
                // AnswerAccessPolicy can ever pass for this value.
                PCon::new(text, AnswerAccessPolicy::new(None, None))
            },
            Signature {
                username: "babman",
                signature: "",
            },
        ),
        String::from(AUTOMATED_ANALYSIS_REASON),
    )
    .unwrap();

    let ctx = AnalysisRender {
        lec_id: lec.into_pcon::<u64, NoPolicy>(),
        question_id,
        result: Some(result),
    };
    PConTemplate::render("admin/analysis", &ctx, context).unwrap()
}
