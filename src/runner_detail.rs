use crate::model::{AssertionResult, HttpMethod};
use crate::{privacy, runner};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunnerTimingDetail {
    pub(crate) prepare_ms: u64,
    pub(crate) waiting_ms: u64,
    pub(crate) download_ms: u64,
    pub(crate) total_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RunnerAssertionOutcome {
    Pass,
    Fail,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunnerAssertionDetail {
    pub(crate) index: usize,
    pub(crate) outcome: RunnerAssertionOutcome,
    pub(crate) message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunnerResultRow {
    pub(crate) collection: String,
    pub(crate) request: String,
    pub(crate) request_name: String,
    pub(crate) row_label: String,
    pub(crate) method: HttpMethod,
    pub(crate) url: String,
    pub(crate) status: String,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) timing: RunnerTimingDetail,
    pub(crate) assertions: Vec<RunnerAssertionDetail>,
    pub(crate) extracted_count: usize,
    pub(crate) extractor_misses: Vec<String>,
    pub(crate) note: String,
}

pub(crate) fn rows_from_result(result: &runner::CollectionRunResult) -> Vec<RunnerResultRow> {
    let mut rows = Vec::new();
    for iteration in &result.iterations {
        let row_label = runner_data_row_label(iteration.index, &iteration.data);
        for request in &iteration.requests {
            rows.push(row_from_request_result(&row_label, request));
        }
    }
    rows
}

pub(crate) fn row_from_progress(progress: &runner::RunnerRequestProgress) -> RunnerResultRow {
    let row_label = runner_data_row_label(progress.iteration_index, &progress.data);
    let assertions = assertion_details(&progress.assertions);
    let note = result_note(
        assertion_counts(&assertions),
        progress.extracted_count,
        progress.extractor_misses.len(),
    );

    RunnerResultRow {
        collection: progress.collection.clone(),
        request: format!("{} - {}", row_label, progress.request_name),
        request_name: progress.request_name.clone(),
        row_label,
        method: progress.method.clone(),
        url: privacy::redact_url_query_and_fragment(&progress.url_template),
        status: progress.status.clone(),
        duration_ms: Some(progress.duration_ms),
        timing: RunnerTimingDetail {
            prepare_ms: progress.prepare_ms,
            waiting_ms: progress.waiting_ms,
            download_ms: progress.download_ms,
            total_ms: progress.duration_ms,
        },
        assertions,
        extracted_count: progress.extracted_count,
        extractor_misses: progress.extractor_misses.clone(),
        note,
    }
}

pub(crate) fn summary_text(row: &RunnerResultRow) -> String {
    let (passed, failed, errored) = assertion_counts(&row.assertions);
    let mut lines = vec![
        format!("Request: {}", row.request),
        format!("Collection: {}", empty_dash(&row.collection)),
        format!("Method: {}", row.method),
        format!("URL: {}", empty_dash(&row.url)),
        format!("Status: {}", empty_dash(&row.status)),
        format!("Total time: {}", duration_text(row.duration_ms)),
        format!(
            "Timings: prepare {} ms, waiting {} ms, download {} ms",
            row.timing.prepare_ms, row.timing.waiting_ms, row.timing.download_ms
        ),
        format!(
            "Assertions: {} pass / {} fail / {} error",
            passed, failed, errored
        ),
        format!("Extracted values: {} value(s) hidden", row.extracted_count),
        format!("Extractor misses: {}", row.extractor_misses.len()),
    ];

    if !row.extractor_misses.is_empty() {
        lines.push(format!(
            "Missing extractors: {}",
            row.extractor_misses.join(", ")
        ));
    }

    lines.join("\n")
}

pub(crate) fn duration_text(duration_ms: Option<u64>) -> String {
    duration_ms
        .map(|ms| format!("{} ms", ms))
        .unwrap_or_else(|| "-".to_string())
}

fn row_from_request_result(row_label: &str, request: &runner::RequestRunResult) -> RunnerResultRow {
    let assertions = assertion_details(&request.assertions);
    let note = result_note(
        assertion_counts(&assertions),
        request.extracted.len(),
        request.extractor_misses.len(),
    );

    RunnerResultRow {
        collection: request.folder_path.join(" / "),
        request: format!("{} - {}", row_label, request.request_name),
        request_name: request.request_name.clone(),
        row_label: row_label.to_string(),
        method: request.method.clone(),
        url: privacy::redact_url_query_and_fragment(&request.url_template),
        status: request.response.status.clone(),
        duration_ms: Some(request.response.total_ms),
        timing: RunnerTimingDetail {
            prepare_ms: request.response.prepare_ms,
            waiting_ms: request.response.waiting_ms,
            download_ms: request.response.download_ms,
            total_ms: request.response.total_ms,
        },
        assertions,
        extracted_count: request.extracted.len(),
        extractor_misses: request.extractor_misses.clone(),
        note,
    }
}

fn runner_data_row_label(index: usize, data: &runner::DataRow) -> String {
    if data.is_empty() {
        format!("Row {}", index + 1)
    } else {
        let keys = data.keys().take(3).cloned().collect::<Vec<_>>().join(", ");
        format!("Row {} ({})", index + 1, keys)
    }
}

fn assertion_details(assertions: &[runner::AssertionRunResult]) -> Vec<RunnerAssertionDetail> {
    assertions
        .iter()
        .map(|assertion| match &assertion.result {
            AssertionResult::Pass => RunnerAssertionDetail {
                index: assertion.index,
                outcome: RunnerAssertionOutcome::Pass,
                message: None,
            },
            AssertionResult::Fail(message) => RunnerAssertionDetail {
                index: assertion.index,
                outcome: RunnerAssertionOutcome::Fail,
                message: Some(message.clone()),
            },
            AssertionResult::Error(message) => RunnerAssertionDetail {
                index: assertion.index,
                outcome: RunnerAssertionOutcome::Error,
                message: Some(message.clone()),
            },
        })
        .collect()
}

fn assertion_counts(assertions: &[RunnerAssertionDetail]) -> (usize, usize, usize) {
    assertions
        .iter()
        .fold((0, 0, 0), |acc, assertion| match assertion.outcome {
            RunnerAssertionOutcome::Pass => (acc.0 + 1, acc.1, acc.2),
            RunnerAssertionOutcome::Fail => (acc.0, acc.1 + 1, acc.2),
            RunnerAssertionOutcome::Error => (acc.0, acc.1, acc.2 + 1),
        })
}

fn result_note(
    (passed, failed, errored): (usize, usize, usize),
    extracted_count: usize,
    extractor_miss_count: usize,
) -> String {
    let mut note_parts = vec![format!(
        "{} pass / {} fail / {} err",
        passed, failed, errored
    )];
    if extracted_count > 0 {
        note_parts.push(format!("{} extracted", extracted_count));
    }
    if extractor_miss_count > 0 {
        note_parts.push(format!("{} missed", extractor_miss_count));
    }
    note_parts.join(", ")
}

fn empty_dash(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ResponseData, StoredCookie};

    fn response() -> ResponseData {
        ResponseData {
            body: "super-secret-body-token".to_string(),
            status: "201 Created".to_string(),
            time: "24 ms".to_string(),
            headers: vec![(
                "Authorization".to_string(),
                "Bearer super-secret-header-token".to_string(),
            )],
            set_cookies: vec![StoredCookie {
                name: "session".to_string(),
                value: "super-secret-cookie".to_string(),
                domain: "api.example.test".to_string(),
                path: "/".to_string(),
                expires: None,
                secure: false,
                http_only: true,
            }],
            response_headers_bytes: 10,
            response_body_bytes: 20,
            request_headers_bytes: 30,
            request_body_bytes: 40,
            prepare_ms: 2,
            waiting_ms: 17,
            download_ms: 5,
            total_ms: 24,
        }
    }

    #[test]
    fn rows_from_result_redacts_url_and_keeps_detail_counts() {
        let result = runner::CollectionRunResult {
            iterations: vec![runner::RunIterationResult {
                index: 0,
                data: runner::DataRow::from([("password".to_string(), "hidden".to_string())]),
                requests: vec![runner::RequestRunResult {
                    request_id: "req-1".to_string(),
                    request_name: "Login".to_string(),
                    method: HttpMethod::POST,
                    url_template: "https://api.example.test/login?token=secret#frag".to_string(),
                    folder_path: vec!["Auth".to_string(), "Smoke".to_string()],
                    response: response(),
                    assertions: vec![
                        runner::AssertionRunResult {
                            index: 0,
                            result: AssertionResult::Pass,
                        },
                        runner::AssertionRunResult {
                            index: 1,
                            result: AssertionResult::Fail("status mismatch".to_string()),
                        },
                        runner::AssertionRunResult {
                            index: 2,
                            result: AssertionResult::Error("bad body path".to_string()),
                        },
                    ],
                    extracted: vec![("access_token".to_string(), "extracted-secret".to_string())],
                    extractor_misses: vec!["refresh_token".to_string()],
                }],
            }],
            total_requests: 1,
            passed_assertions: 1,
            failed_assertions: 1,
            errored_assertions: 1,
        };

        let rows = rows_from_result(&result);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];

        assert_eq!(row.collection, "Auth / Smoke");
        assert_eq!(row.request, "Row 1 (password) - Login");
        assert_eq!(row.url, "https://api.example.test/login?...#...");
        assert_eq!(row.timing.prepare_ms, 2);
        assert_eq!(row.timing.waiting_ms, 17);
        assert_eq!(row.timing.download_ms, 5);
        assert_eq!(row.note, "1 pass / 1 fail / 1 err, 1 extracted, 1 missed");
        assert_eq!(row.extractor_misses, vec!["refresh_token"]);
        assert_eq!(row.extracted_count, 1);
    }

    #[test]
    fn summary_text_omits_response_content_and_extracted_values() {
        let result = runner::CollectionRunResult {
            iterations: vec![runner::RunIterationResult {
                index: 0,
                data: runner::DataRow::new(),
                requests: vec![runner::RequestRunResult {
                    request_id: "req-1".to_string(),
                    request_name: "Fetch Profile".to_string(),
                    method: HttpMethod::GET,
                    url_template: "https://api.example.test/profile?api_key=query-secret"
                        .to_string(),
                    folder_path: vec!["Users".to_string()],
                    response: response(),
                    assertions: vec![],
                    extracted: vec![("profile_token".to_string(), "extracted-secret".to_string())],
                    extractor_misses: vec!["profile_id".to_string()],
                }],
            }],
            total_requests: 1,
            passed_assertions: 0,
            failed_assertions: 0,
            errored_assertions: 0,
        };

        let row = rows_from_result(&result).remove(0);
        let summary = summary_text(&row);

        assert!(summary.contains("https://api.example.test/profile?..."));
        assert!(summary.contains("Extracted values: 1 value(s) hidden"));
        assert!(summary.contains("Missing extractors: profile_id"));
        assert!(!summary.contains("query-secret"));
        assert!(!summary.contains("super-secret-body-token"));
        assert!(!summary.contains("super-secret-header-token"));
        assert!(!summary.contains("super-secret-cookie"));
        assert!(!summary.contains("extracted-secret"));
    }

    #[test]
    fn row_from_progress_preserves_live_detail_without_values() {
        let progress = runner::RunnerRequestProgress {
            iteration_index: 1,
            data: runner::DataRow::from([("user".to_string(), "alice".to_string())]),
            completed_requests: 2,
            total_requests: 3,
            collection: "Auth".to_string(),
            request_name: "Login".to_string(),
            method: HttpMethod::POST,
            url_template: "https://api.example.test/login?password=secret".to_string(),
            status: "500 Internal Server Error".to_string(),
            duration_ms: 42,
            prepare_ms: 3,
            waiting_ms: 37,
            download_ms: 2,
            passed_assertions: 0,
            failed_assertions: 1,
            errored_assertions: 0,
            assertions: vec![runner::AssertionRunResult {
                index: 0,
                result: AssertionResult::Fail("status mismatch".to_string()),
            }],
            extracted_count: 1,
            extractor_miss_count: 1,
            extractor_misses: vec!["token".to_string()],
        };

        let row = row_from_progress(&progress);

        assert_eq!(row.request, "Row 2 (user) - Login");
        assert_eq!(row.url, "https://api.example.test/login?...");
        assert_eq!(row.note, "0 pass / 1 fail / 0 err, 1 extracted, 1 missed");
        assert_eq!(row.assertions[0].outcome, RunnerAssertionOutcome::Fail);
        assert_eq!(row.extractor_misses, vec!["token"]);
    }
}
