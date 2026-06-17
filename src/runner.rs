//! Collection runner engine. This module is intentionally UI-free: it
//! flattens folders into ordered requests, overlays optional data rows
//! onto an environment, executes requests sequentially through the
//! existing network layer, and returns structured results that can feed
//! a future UI or report exporter.

#![allow(dead_code)]

use crate::assertion;
use crate::cookies;
use crate::extract;
use crate::model::{
    AppSettings, AssertionResult, Environment, ExtractorSource, Folder, HttpMethod, KvRow, Request,
    ResponseData,
};
use crate::net;
use std::collections::BTreeMap;

pub type DataRow = BTreeMap<String, String>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerOptions {
    /// Optional root folder id. When omitted, every top-level folder is
    /// included in app order.
    pub folder_id: Option<String>,
    /// Optional data rows. One full collection iteration runs per row.
    /// Empty means one iteration with the base environment unchanged.
    pub data_rows: Vec<DataRow>,
}

pub struct CollectionRunResult {
    pub iterations: Vec<RunIterationResult>,
    pub total_requests: usize,
    pub passed_assertions: usize,
    pub failed_assertions: usize,
    pub errored_assertions: usize,
}

pub enum CollectionRunEvent {
    RequestFinished(Box<RunnerRequestProgress>),
    Finished(CollectionRunResult),
}

#[derive(Clone, Debug)]
pub struct RunnerRequestProgress {
    pub iteration_index: usize,
    pub data: DataRow,
    pub completed_requests: usize,
    pub total_requests: usize,
    pub collection: String,
    pub request_name: String,
    pub method: HttpMethod,
    pub url_template: String,
    pub status: String,
    pub duration_ms: u64,
    pub prepare_ms: u64,
    pub waiting_ms: u64,
    pub download_ms: u64,
    pub passed_assertions: usize,
    pub failed_assertions: usize,
    pub errored_assertions: usize,
    pub assertions: Vec<AssertionRunResult>,
    pub extracted_count: usize,
    pub extractor_miss_count: usize,
    pub extractor_misses: Vec<String>,
}

pub struct RunIterationResult {
    pub index: usize,
    pub data: DataRow,
    pub requests: Vec<RequestRunResult>,
}

pub struct RequestRunResult {
    pub request_id: String,
    pub request_name: String,
    pub method: HttpMethod,
    pub url_template: String,
    pub folder_path: Vec<String>,
    pub response: ResponseData,
    pub assertions: Vec<AssertionRunResult>,
    pub extracted: Vec<(String, String)>,
    pub extractor_misses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssertionRunResult {
    pub index: usize,
    pub result: AssertionResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataParseError {
    Json(String),
    Csv(String),
    UnsupportedFormat(String),
}

/// Collect requests from a folder tree in depth-first UI order.
pub fn collect_requests(folders: &[Folder], folder_id: Option<&str>) -> Vec<RunnerRequest> {
    let roots: Vec<&Folder> = match folder_id {
        Some(id) => find_folder(folders, id).into_iter().collect(),
        None => folders.iter().collect(),
    };
    let mut out = Vec::new();
    for folder in roots {
        collect_folder_requests(folder, Vec::new(), &mut out);
    }
    out
}

/// Parse simple JSON or CSV runner data into string variable maps.
///
/// JSON accepts either an object (`{"a":1}`) as one row or an array of
/// objects. CSV expects a header row and comma-separated fields; quoted
/// fields and escaped quotes are supported.
pub fn parse_data_rows(input: &str, format_hint: &str) -> Result<Vec<DataRow>, DataParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    match format_hint
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => parse_json_rows(trimmed),
        "csv" => parse_csv_rows(trimmed),
        other => Err(DataParseError::UnsupportedFormat(other.to_string())),
    }
}

pub async fn run_collection(
    folders: &[Folder],
    options: RunnerOptions,
    settings: &AppSettings,
    env: Option<Environment>,
) -> CollectionRunResult {
    run_collection_inner(folders, options, settings, env, None).await
}

pub async fn run_collection_with_progress(
    folders: &[Folder],
    options: RunnerOptions,
    settings: &AppSettings,
    env: Option<Environment>,
    progress: std::sync::mpsc::Sender<CollectionRunEvent>,
) {
    let result =
        run_collection_inner(folders, options, settings, env, Some(progress.clone())).await;
    let _ = progress.send(CollectionRunEvent::Finished(result));
}

async fn run_collection_inner(
    folders: &[Folder],
    options: RunnerOptions,
    settings: &AppSettings,
    env: Option<Environment>,
    progress: Option<std::sync::mpsc::Sender<CollectionRunEvent>>,
) -> CollectionRunResult {
    let requests = collect_requests(folders, options.folder_id.as_deref());
    let rows = if options.data_rows.is_empty() {
        vec![DataRow::new()]
    } else {
        options.data_rows
    };
    let total_runs = requests.len().saturating_mul(rows.len());
    let client = net::build_client(settings);
    let max_body_bytes = (settings.max_body_mb as usize).saturating_mul(1024 * 1024);

    let mut iterations = Vec::with_capacity(rows.len());
    let mut completed_runs = 0usize;
    for (index, data) in rows.into_iter().enumerate() {
        let mut run_env = Some(env.clone().unwrap_or_else(runner_environment));
        overlay_data_row(&mut run_env, &data);

        let mut request_results = Vec::with_capacity(requests.len());
        for runner_request in &requests {
            let response = net::execute_request_async(
                client.clone(),
                runner_request.request.clone(),
                run_env.clone(),
                max_body_bytes,
                None,
            )
            .await;

            if let Some(e) = run_env.as_mut() {
                for cookie in response.set_cookies.clone() {
                    cookies::upsert(&mut e.cookies, cookie);
                }
                cookies::prune(&mut e.cookies);
            }

            let (extracted, extractor_misses) =
                apply_extractors(&runner_request.request, &response, run_env.as_mut());
            let assertions = evaluate_assertions(&runner_request.request, &response);
            completed_runs += 1;

            if let Some(progress) = &progress {
                let (passed, failed, errored) = assertion_counts(&assertions);
                let _ = progress.send(CollectionRunEvent::RequestFinished(Box::new(
                    RunnerRequestProgress {
                        iteration_index: index,
                        data: data.clone(),
                        completed_requests: completed_runs,
                        total_requests: total_runs,
                        collection: runner_request.folder_path.join(" / "),
                        request_name: runner_request.request.name.clone(),
                        method: runner_request.request.method.clone(),
                        url_template: runner_request.request.url.clone(),
                        status: response.status.clone(),
                        duration_ms: response.total_ms,
                        prepare_ms: response.prepare_ms,
                        waiting_ms: response.waiting_ms,
                        download_ms: response.download_ms,
                        passed_assertions: passed,
                        failed_assertions: failed,
                        errored_assertions: errored,
                        assertions: assertions.clone(),
                        extracted_count: extracted.len(),
                        extractor_miss_count: extractor_misses.len(),
                        extractor_misses: extractor_misses.clone(),
                    },
                )));
            }

            let mut stored_response = response;
            stored_response.body.clear();
            stored_response.headers.clear();
            stored_response.set_cookies.clear();

            request_results.push(RequestRunResult {
                request_id: runner_request.request.id.clone(),
                request_name: runner_request.request.name.clone(),
                method: runner_request.request.method.clone(),
                url_template: runner_request.request.url.clone(),
                folder_path: runner_request.folder_path.clone(),
                response: stored_response,
                assertions,
                extracted,
                extractor_misses,
            });
        }

        iterations.push(RunIterationResult {
            index,
            data,
            requests: request_results,
        });
    }

    summarize(iterations)
}

#[derive(Clone, Debug)]
pub struct RunnerRequest {
    pub request: Request,
    pub folder_path: Vec<String>,
}

fn collect_folder_requests(folder: &Folder, mut path: Vec<String>, out: &mut Vec<RunnerRequest>) {
    path.push(folder.name.clone());
    for request in &folder.requests {
        out.push(RunnerRequest {
            request: request.clone(),
            folder_path: path.clone(),
        });
    }
    for child in &folder.subfolders {
        collect_folder_requests(child, path.clone(), out);
    }
}

fn find_folder<'a>(folders: &'a [Folder], id: &str) -> Option<&'a Folder> {
    for folder in folders {
        if folder.id == id {
            return Some(folder);
        }
        if let Some(found) = find_folder(&folder.subfolders, id) {
            return Some(found);
        }
    }
    None
}

fn parse_json_rows(input: &str) -> Result<Vec<DataRow>, DataParseError> {
    let value = serde_json::from_str::<serde_json::Value>(input)
        .map_err(|e| DataParseError::Json(e.to_string()))?;
    match value {
        serde_json::Value::Object(obj) => Ok(vec![json_object_to_row(obj)]),
        serde_json::Value::Array(rows) => rows
            .into_iter()
            .map(|row| match row {
                serde_json::Value::Object(obj) => Ok(json_object_to_row(obj)),
                other => Err(DataParseError::Json(format!(
                    "expected object row, got {}",
                    json_type(&other)
                ))),
            })
            .collect(),
        other => Err(DataParseError::Json(format!(
            "expected object or array of objects, got {}",
            json_type(&other)
        ))),
    }
}

fn json_object_to_row(obj: serde_json::Map<String, serde_json::Value>) -> DataRow {
    obj.into_iter()
        .map(|(key, value)| (key, json_value_to_string(value)))
        .collect()
}

fn json_value_to_string(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn parse_csv_rows(input: &str) -> Result<Vec<DataRow>, DataParseError> {
    let records = parse_csv_records(input)?;
    let Some(headers) = records.first() else {
        return Ok(Vec::new());
    };
    if headers.is_empty() {
        return Err(DataParseError::Csv("missing header row".to_string()));
    }

    let mut rows = Vec::new();
    for (line_index, record) in records.iter().enumerate().skip(1) {
        if record.len() > headers.len() {
            return Err(DataParseError::Csv(format!(
                "row {} has more fields than headers",
                line_index + 1
            )));
        }
        let mut row = DataRow::new();
        for (i, header) in headers.iter().enumerate() {
            row.insert(header.clone(), record.get(i).cloned().unwrap_or_default());
        }
        rows.push(row);
    }
    Ok(rows)
}

fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, DataParseError> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                record.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            other => field.push(other),
        }
    }

    if in_quotes {
        return Err(DataParseError::Csv("unterminated quoted field".to_string()));
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

fn overlay_data_row(env: &mut Option<Environment>, data: &DataRow) {
    if data.is_empty() {
        return;
    }
    let env = env.get_or_insert_with(runner_environment);
    for (key, value) in data {
        upsert_var(env, key, value);
    }
}

fn runner_environment() -> Environment {
    Environment {
        id: "runner-data".to_string(),
        name: "Runner Data".to_string(),
        variables: vec![],
        cookies: vec![],
    }
}

fn apply_extractors(
    request: &Request,
    response: &ResponseData,
    env: Option<&mut Environment>,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut writes = Vec::new();
    let mut missed = Vec::new();
    for extractor in &request.extractors {
        if !extractor.enabled {
            continue;
        }
        let variable = extractor.variable.trim();
        if variable.is_empty() {
            continue;
        }
        let value = match extractor.source {
            ExtractorSource::Body => {
                extract::eval_body_path(&response.body, extractor.expression.trim())
            }
            ExtractorSource::Header => response
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(extractor.expression.trim()))
                .map(|(_, v)| v.clone()),
            ExtractorSource::Status => Some(
                response
                    .status
                    .split_whitespace()
                    .next()
                    .unwrap_or(&response.status)
                    .to_string(),
            ),
        };
        match value {
            Some(value) => writes.push((variable.to_string(), value)),
            None => missed.push(variable.to_string()),
        }
    }

    if let Some(env) = env {
        for (key, value) in &writes {
            upsert_var(env, key, value);
        }
    }
    (writes, missed)
}

fn upsert_var(env: &mut Environment, key: &str, value: &str) {
    match env.variables.iter_mut().find(|row| row.key == key) {
        Some(existing) => {
            existing.enabled = true;
            existing.value = value.to_string();
        }
        None => env.variables.push(KvRow::new(key, value)),
    }
}

fn evaluate_assertions(request: &Request, response: &ResponseData) -> Vec<AssertionRunResult> {
    request
        .assertions
        .iter()
        .enumerate()
        .filter(|(_, assertion)| assertion.enabled)
        .map(|(index, assertion)| AssertionRunResult {
            index,
            result: assertion::evaluate(
                assertion,
                &response.status,
                &response.body,
                &response.headers,
            ),
        })
        .collect()
}

fn summarize(iterations: Vec<RunIterationResult>) -> CollectionRunResult {
    let mut total_requests = 0;
    let mut passed_assertions = 0;
    let mut failed_assertions = 0;
    let mut errored_assertions = 0;

    for iteration in &iterations {
        total_requests += iteration.requests.len();
        for request in &iteration.requests {
            for assertion in &request.assertions {
                match assertion.result {
                    AssertionResult::Pass => passed_assertions += 1,
                    AssertionResult::Fail(_) => failed_assertions += 1,
                    AssertionResult::Error(_) => errored_assertions += 1,
                }
            }
        }
    }

    CollectionRunResult {
        iterations,
        total_requests,
        passed_assertions,
        failed_assertions,
        errored_assertions,
    }
}

fn assertion_counts(assertions: &[AssertionRunResult]) -> (usize, usize, usize) {
    assertions
        .iter()
        .fold((0, 0, 0), |acc, assertion| match assertion.result {
            AssertionResult::Pass => (acc.0 + 1, acc.1, acc.2),
            AssertionResult::Fail(_) => (acc.0, acc.1 + 1, acc.2),
            AssertionResult::Error(_) => (acc.0, acc.1, acc.2 + 1),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AssertionOp, AssertionSource, Auth, ExtractorSource, HttpMethod, ResponseAssertion,
        ResponseExtractor, StoredCookie,
    };

    fn request(id: &str, name: &str) -> Request {
        Request {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            method: HttpMethod::GET,
            url: "http://example.test".to_string(),
            query_params: vec![],
            path_params: vec![],
            headers: vec![],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
            assertions: vec![],
            source: None,
        }
    }

    fn response(status: &str, body: &str) -> ResponseData {
        ResponseData {
            body: body.to_string(),
            status: status.to_string(),
            time: "1 ms".to_string(),
            headers: vec![("X-Trace".to_string(), "abc".to_string())],
            set_cookies: vec![],
            response_headers_bytes: 0,
            response_body_bytes: body.len(),
            request_headers_bytes: 0,
            request_body_bytes: 0,
            prepare_ms: 0,
            waiting_ms: 0,
            download_ms: 0,
            total_ms: 1,
        }
    }

    #[test]
    fn collects_requests_depth_first_with_folder_path() {
        let folders = vec![Folder {
            id: "root".to_string(),
            name: "Root".to_string(),
            requests: vec![request("a", "A")],
            subfolders: vec![Folder {
                id: "child".to_string(),
                name: "Child".to_string(),
                requests: vec![request("b", "B")],
                subfolders: vec![],
                description: String::new(),
                sync: crate::model::SyncConfig::default(),
            }],
            description: String::new(),
            sync: crate::model::SyncConfig::default(),
        }];

        let all = collect_requests(&folders, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].request.id, "a");
        assert_eq!(all[0].folder_path, vec!["Root"]);
        assert_eq!(all[1].request.id, "b");
        assert_eq!(all[1].folder_path, vec!["Root", "Child"]);

        let child = collect_requests(&folders, Some("child"));
        assert_eq!(child.len(), 1);
        assert_eq!(child[0].request.id, "b");
    }

    #[test]
    fn parses_json_object_and_array_rows() {
        let rows =
            parse_data_rows(r#"[{"id":1,"name":"Ada"},{"id":2,"active":true}]"#, "json").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("id").unwrap(), "1");
        assert_eq!(rows[0].get("name").unwrap(), "Ada");
        assert_eq!(rows[1].get("active").unwrap(), "true");

        let row = parse_data_rows(r#"{"nested":{"x":1},"empty":null}"#, ".json").unwrap();
        assert_eq!(row[0].get("nested").unwrap(), r#"{"x":1}"#);
        assert_eq!(row[0].get("empty").unwrap(), "");
    }

    #[test]
    fn parses_csv_rows_with_quotes() {
        let rows = parse_data_rows(
            "id,name,note\n1,Ada,\"hello, world\"\n2,Bob,\"a \"\"quote\"\"\"",
            "csv",
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("note").unwrap(), "hello, world");
        assert_eq!(rows[1].get("note").unwrap(), "a \"quote\"");
    }

    #[test]
    fn overlays_data_rows_into_env() {
        let mut env = Some(Environment {
            id: "env".to_string(),
            name: "Env".to_string(),
            variables: vec![KvRow::new("id", "old")],
            cookies: vec![],
        });
        let data = DataRow::from([
            ("id".to_string(), "new".to_string()),
            ("name".to_string(), "Ada".to_string()),
        ]);

        overlay_data_row(&mut env, &data);
        let env = env.unwrap();
        assert_eq!(
            env.variables.iter().find(|v| v.key == "id").unwrap().value,
            "new"
        );
        assert_eq!(
            env.variables
                .iter()
                .find(|v| v.key == "name")
                .unwrap()
                .value,
            "Ada"
        );
    }

    #[test]
    fn overlays_data_rows_create_runner_env_when_missing() {
        let mut env = None;
        let data = DataRow::from([("token".to_string(), "abc".to_string())]);

        overlay_data_row(&mut env, &data);

        let env = env.unwrap();
        assert_eq!(env.name, "Runner Data");
        assert_eq!(
            env.variables
                .iter()
                .find(|v| v.key == "token")
                .unwrap()
                .value,
            "abc"
        );
    }

    #[test]
    fn evaluates_enabled_assertions_only() {
        let mut req = request("a", "A");
        req.assertions = vec![
            ResponseAssertion {
                enabled: true,
                source: AssertionSource::Status,
                expression: String::new(),
                op: AssertionOp::Equals,
                expected: "200".to_string(),
            },
            ResponseAssertion {
                enabled: false,
                source: AssertionSource::Header,
                expression: "X-Trace".to_string(),
                op: AssertionOp::Equals,
                expected: "abc".to_string(),
            },
        ];

        let results = evaluate_assertions(&req, &response("200 OK", "{}"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);
        assert_eq!(results[0].result, AssertionResult::Pass);
    }

    #[test]
    fn applies_extractors_and_records_misses() {
        let mut req = request("a", "A");
        req.extractors = vec![
            ResponseExtractor {
                enabled: true,
                variable: "token".to_string(),
                source: ExtractorSource::Body,
                expression: "data.token".to_string(),
            },
            ResponseExtractor {
                enabled: true,
                variable: "missing".to_string(),
                source: ExtractorSource::Header,
                expression: "X-Missing".to_string(),
            },
        ];
        let mut env = Environment {
            id: "env".to_string(),
            name: "Env".to_string(),
            variables: vec![],
            cookies: vec![StoredCookie {
                name: "sid".to_string(),
                value: "old".to_string(),
                domain: "example.test".to_string(),
                path: "/".to_string(),
                expires: None,
                secure: false,
                http_only: false,
            }],
        };

        let (writes, misses) = apply_extractors(
            &req,
            &response("200 OK", r#"{"data":{"token":"secret"}}"#),
            Some(&mut env),
        );

        assert_eq!(writes, vec![("token".to_string(), "secret".to_string())]);
        assert_eq!(misses, vec!["missing".to_string()]);
        assert_eq!(
            env.variables
                .iter()
                .find(|row| row.key == "token")
                .unwrap()
                .value,
            "secret"
        );
    }

    #[test]
    fn summarizes_run_results() {
        let run = summarize(vec![RunIterationResult {
            index: 0,
            data: DataRow::new(),
            requests: vec![RequestRunResult {
                request_id: "a".to_string(),
                request_name: "A".to_string(),
                method: HttpMethod::GET,
                url_template: "http://example.test".to_string(),
                folder_path: vec!["Root".to_string()],
                response: response("200 OK", "{}"),
                assertions: vec![
                    AssertionRunResult {
                        index: 0,
                        result: AssertionResult::Pass,
                    },
                    AssertionRunResult {
                        index: 1,
                        result: AssertionResult::Fail("nope".to_string()),
                    },
                ],
                extracted: vec![],
                extractor_misses: vec![],
            }],
        }]);

        assert_eq!(run.total_requests, 1);
        assert_eq!(run.passed_assertions, 1);
        assert_eq!(run.failed_assertions, 1);
        assert_eq!(run.errored_assertions, 0);
    }
}
