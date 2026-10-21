//! Headless CI runner. Reuses the UI-free collection runner engine to
//! execute a saved scope from the command line, write reports, and exit
//! with a status code CI can gate on. No egui/eframe on this path.

use crate::model::{AppState, Environment, Folder};
use crate::privacy;
use crate::runner::{self, RunnerOptions};
use crate::runner_detail::{self, RunnerAssertionOutcome, RunnerResultRow};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Exit code for a usage or configuration error (bad args, scope not
/// found, unreadable workspace). Distinct from `1` so CI can tell a
/// real test failure from a misconfigured invocation.
const EXIT_USAGE: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeKind {
    Folder,
    Request,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailOn {
    Any,
    Assertions,
    Requests,
    Never,
}

struct HeadlessArgs {
    workspace: Option<PathBuf>,
    scope: Option<String>,
    kind: ScopeKind,
    env: Option<String>,
    data_path: Option<PathBuf>,
    data_format: Option<String>,
    report_json: Option<PathBuf>,
    report_csv: Option<PathBuf>,
    report_html: Option<PathBuf>,
    fail_on: FailOn,
}

const USAGE: &str = "\
Usage: rusty-requester run [SCOPE] [OPTIONS]

Runs a saved collection/folder/request scope without the GUI.

Arguments:
  SCOPE                 Collection/folder id or path (e.g. \"API/Auth\").
                        Omit to run every top-level collection.

Options:
  --kind <folder|request>   How to interpret SCOPE (default: folder).
  --env <name|id>           Environment to run against. Defaults to the
                            workspace's active environment.
  --data <file>             CSV or JSON data rows; one iteration per row.
  --data-format <csv|json>  Override the data format (default: by extension).
  --report-json <file>      Write a JSON report.
  --report-csv <file>       Write a CSV report.
  --report-html <file>      Write an HTML report.
  --fail-on <mode>          Nonzero-exit policy: any (default), assertions,
                            requests, none.
  --workspace <file>        data.json to load (default: the GUI's path).
  -h, --help                Show this help.";

/// Resolved run scope: either a folder id to pass through, every
/// top-level collection, or a single request wrapped in a synthetic
/// folder so the existing engine can run it unchanged.
enum ResolvedScope {
    All,
    Folder(String),
    Request(Box<Folder>),
}

/// Entry point for `rusty-requester run …`. Returns a process exit code.
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{}", USAGE);
        return 0;
    }
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}\n\n{}", e, USAGE);
            return EXIT_USAGE;
        }
    };
    match execute(parsed) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {}", e);
            EXIT_USAGE
        }
    }
}

fn execute(args: HeadlessArgs) -> Result<i32, String> {
    let path = args
        .workspace
        .clone()
        .unwrap_or_else(default_workspace_path);
    let state = load_workspace(&path)?;

    let scope = resolve_scope(&state.folders, args.scope.as_deref(), args.kind)?;
    let (folders, folder_id): (Vec<Folder>, Option<String>) = match scope {
        ResolvedScope::All => (state.folders.clone(), None),
        ResolvedScope::Folder(id) => (state.folders.clone(), Some(id)),
        ResolvedScope::Request(wrapper) => (vec![*wrapper], None),
    };

    let env = resolve_env(&state, args.env.as_deref())?;

    let data_rows = match &args.data_path {
        Some(p) => load_data_rows(p, args.data_format.as_deref())?,
        None => Vec::new(),
    };

    let options = RunnerOptions {
        folder_id,
        data_rows,
    };

    let runtime = crate::net::build_runtime();
    let result = runtime.block_on(runner::run_collection(
        &folders,
        options,
        &state.settings,
        env,
    ));

    let report = build_report(&result);
    write_reports(&args, &report)?;
    print_summary(&report);

    Ok(exit_code(&report.summary, args.fail_on))
}

fn parse_args(args: &[String]) -> Result<HeadlessArgs, String> {
    let mut out = HeadlessArgs {
        workspace: None,
        scope: None,
        kind: ScopeKind::Folder,
        env: None,
        data_path: None,
        data_format: None,
        report_json: None,
        report_csv: None,
        report_html: None,
        fail_on: FailOn::Any,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut take = |name: &str| -> Result<String, String> {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{} needs a value", name))
        };
        match arg.as_str() {
            "--kind" => {
                out.kind = match take("--kind")?.as_str() {
                    "folder" | "collection" => ScopeKind::Folder,
                    "request" => ScopeKind::Request,
                    other => return Err(format!("unknown --kind '{}'", other)),
                }
            }
            "--env" => out.env = Some(take("--env")?),
            "--data" => out.data_path = Some(PathBuf::from(take("--data")?)),
            "--data-format" => out.data_format = Some(take("--data-format")?),
            "--report-json" => out.report_json = Some(PathBuf::from(take("--report-json")?)),
            "--report-csv" => out.report_csv = Some(PathBuf::from(take("--report-csv")?)),
            "--report-html" => out.report_html = Some(PathBuf::from(take("--report-html")?)),
            "--workspace" => out.workspace = Some(PathBuf::from(take("--workspace")?)),
            "--fail-on" => {
                out.fail_on = match take("--fail-on")?.as_str() {
                    "any" => FailOn::Any,
                    "assertions" => FailOn::Assertions,
                    "requests" => FailOn::Requests,
                    "none" | "never" => FailOn::Never,
                    other => return Err(format!("unknown --fail-on '{}'", other)),
                }
            }
            other if other.starts_with("--") => return Err(format!("unknown option '{}'", other)),
            other => {
                if out.scope.is_some() {
                    return Err("only one SCOPE argument is allowed".to_string());
                }
                out.scope = Some(other.to_string());
            }
        }
    }
    Ok(out)
}

fn default_workspace_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rusty-requester")
        .join("data.json")
}

fn load_workspace(path: &Path) -> Result<AppState, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read workspace {}: {}", path.display(), e))?;
    serde_json::from_str(&data).map_err(|e| format!("could not parse {}: {}", path.display(), e))
}

fn resolve_env(state: &AppState, selector: Option<&str>) -> Result<Option<Environment>, String> {
    match selector {
        Some(sel) => state
            .environments
            .iter()
            .find(|e| e.id == sel || e.name == sel)
            .cloned()
            .map(Some)
            .ok_or_else(|| format!("no environment matches '{}'", sel)),
        None => Ok(state
            .active_env_id
            .as_ref()
            .and_then(|id| state.environments.iter().find(|e| &e.id == id).cloned())),
    }
}

fn load_data_rows(
    path: &Path,
    format_override: Option<&str>,
) -> Result<Vec<runner::DataRow>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read data file {}: {}", path.display(), e))?;
    let hint = format_override
        .map(|s| s.to_string())
        .or_else(|| {
            path.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| "data file has no extension; pass --data-format".to_string())?;
    runner::parse_data_rows(&content, &hint)
        .map_err(|e| format!("could not parse data rows: {:?}", e))
}

fn resolve_scope(
    folders: &[Folder],
    scope: Option<&str>,
    kind: ScopeKind,
) -> Result<ResolvedScope, String> {
    let Some(scope) = scope else {
        return Ok(ResolvedScope::All);
    };
    match kind {
        ScopeKind::Folder => find_folder_id(folders, scope)
            .map(ResolvedScope::Folder)
            .ok_or_else(|| format!("no collection or folder matches '{}'", scope)),
        ScopeKind::Request => find_request_wrapper(folders, scope)
            .map(|f| ResolvedScope::Request(Box::new(f)))
            .ok_or_else(|| format!("no request matches '{}'", scope)),
    }
}

/// Find a folder by id first, then by its `/`-separated name path.
fn find_folder_id(folders: &[Folder], scope: &str) -> Option<String> {
    fn by_id(folders: &[Folder], id: &str) -> Option<String> {
        for f in folders {
            if f.id == id {
                return Some(f.id.clone());
            }
            if let Some(found) = by_id(&f.subfolders, id) {
                return Some(found);
            }
        }
        None
    }
    fn by_path<'a>(folders: &'a [Folder], segments: &[&str]) -> Option<&'a Folder> {
        let (head, rest) = segments.split_first()?;
        let folder = folders.iter().find(|f| f.name == *head)?;
        if rest.is_empty() {
            Some(folder)
        } else {
            by_path(&folder.subfolders, rest)
        }
    }
    if let Some(id) = by_id(folders, scope) {
        return Some(id);
    }
    let segments: Vec<&str> = scope.split('/').map(|s| s.trim()).collect();
    by_path(folders, &segments).map(|f| f.id.clone())
}

/// Find a single request by id or `/`-separated path and wrap it in a
/// synthetic folder named after its collection path so the engine runs
/// exactly one request while reports still show where it lives.
fn find_request_wrapper(folders: &[Folder], scope: &str) -> Option<Folder> {
    fn by_id(folders: &[Folder], path: &mut Vec<String>, id: &str) -> Option<(Vec<String>, usize)> {
        for f in folders {
            path.push(f.name.clone());
            if let Some(pos) = f.requests.iter().position(|r| r.id == id) {
                return Some((path.clone(), pos));
            }
            if let Some(found) = by_id(&f.subfolders, path, id) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    let wrap = |path: &[String], request: crate::model::Request| Folder {
        id: String::new(),
        name: path.join(" / "),
        requests: vec![request],
        subfolders: vec![],
        description: String::new(),
        sync: crate::model::SyncConfig::default(),
    };

    let mut path = Vec::new();
    if let Some((found_path, pos)) = by_id(folders, &mut path, scope) {
        let folder = folder_at(folders, &found_path)?;
        return Some(wrap(&found_path, folder.requests[pos].clone()));
    }

    let segments: Vec<&str> = scope.split('/').map(|s| s.trim()).collect();
    let (name, folder_path) = segments.split_last()?;
    let folder = folder_by_names(folders, folder_path)?;
    let request = folder.requests.iter().find(|r| &r.name == name)?;
    Some(wrap(
        &folder_path
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        request.clone(),
    ))
}

fn folder_at<'a>(folders: &'a [Folder], names: &[String]) -> Option<&'a Folder> {
    let (head, rest) = names.split_first()?;
    let folder = folders.iter().find(|f| &f.name == head)?;
    if rest.is_empty() {
        Some(folder)
    } else {
        folder_at(&folder.subfolders, rest)
    }
}

fn folder_by_names<'a>(folders: &'a [Folder], names: &[&str]) -> Option<&'a Folder> {
    let (head, rest) = names.split_first()?;
    let folder = folders.iter().find(|f| f.name == *head)?;
    if rest.is_empty() {
        Some(folder)
    } else {
        folder_by_names(&folder.subfolders, rest)
    }
}

#[derive(Serialize)]
struct Report {
    summary: ReportSummary,
    results: Vec<ReportRow>,
}

#[derive(Serialize)]
struct ReportSummary {
    total_requests: usize,
    failed_requests: usize,
    passed_assertions: usize,
    failed_assertions: usize,
    errored_assertions: usize,
}

#[derive(Serialize)]
struct ReportRow {
    collection: String,
    request: String,
    method: String,
    url: String,
    status: String,
    duration_ms: Option<u64>,
    passed_assertions: usize,
    failed_assertions: usize,
    errored_assertions: usize,
    extracted_count: usize,
    extractor_misses: Vec<String>,
    assertions: Vec<ReportAssertion>,
}

#[derive(Serialize)]
struct ReportAssertion {
    index: usize,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn build_report(result: &runner::CollectionRunResult) -> Report {
    let rows = runner_detail::rows_from_result(result);
    let failed_requests = rows.iter().filter(|r| request_failed(&r.status)).count();
    let results = rows.iter().map(report_row).collect();
    Report {
        summary: ReportSummary {
            total_requests: result.total_requests,
            failed_requests,
            passed_assertions: result.passed_assertions,
            failed_assertions: result.failed_assertions,
            errored_assertions: result.errored_assertions,
        },
        results,
    }
}

fn report_row(row: &RunnerResultRow) -> ReportRow {
    let (passed, failed, errored) = assertion_counts(row);
    ReportRow {
        collection: row.collection.clone(),
        request: row.request.clone(),
        method: row.method.to_string(),
        url: row.url.clone(),
        status: row.status.clone(),
        duration_ms: row.duration_ms,
        passed_assertions: passed,
        failed_assertions: failed,
        errored_assertions: errored,
        extracted_count: row.extracted_count,
        extractor_misses: row.extractor_misses.clone(),
        assertions: row
            .assertions
            .iter()
            .map(|a| ReportAssertion {
                index: a.index,
                outcome: outcome_label(&a.outcome),
                message: a.message.clone(),
            })
            .collect(),
    }
}

fn outcome_label(outcome: &RunnerAssertionOutcome) -> &'static str {
    match outcome {
        RunnerAssertionOutcome::Pass => "pass",
        RunnerAssertionOutcome::Fail => "fail",
        RunnerAssertionOutcome::Error => "error",
    }
}

fn assertion_counts(row: &RunnerResultRow) -> (usize, usize, usize) {
    row.assertions
        .iter()
        .fold((0, 0, 0), |acc, a| match a.outcome {
            RunnerAssertionOutcome::Pass => (acc.0 + 1, acc.1, acc.2),
            RunnerAssertionOutcome::Fail => (acc.0, acc.1 + 1, acc.2),
            RunnerAssertionOutcome::Error => (acc.0, acc.1, acc.2 + 1),
        })
}

/// A request counts as failed unless its status code is 2xx/3xx. A
/// network error leaves a non-numeric status ("Failed"), also a failure.
fn request_failed(status: &str) -> bool {
    match status
        .split_whitespace()
        .next()
        .and_then(|c| c.parse::<u16>().ok())
    {
        Some(code) => !(200..400).contains(&code),
        None => true,
    }
}

fn exit_code(summary: &ReportSummary, fail_on: FailOn) -> i32 {
    let assertions_bad = summary.failed_assertions + summary.errored_assertions > 0;
    let requests_bad = summary.failed_requests > 0;
    let fail = match fail_on {
        FailOn::Any => assertions_bad || requests_bad,
        FailOn::Assertions => assertions_bad,
        FailOn::Requests => requests_bad,
        FailOn::Never => false,
    };
    i32::from(fail)
}

fn write_reports(args: &HeadlessArgs, report: &Report) -> Result<(), String> {
    if let Some(path) = &args.report_json {
        let body = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
        write_file(path, &body)?;
    }
    if let Some(path) = &args.report_csv {
        write_file(path, &report_csv(report))?;
    }
    if let Some(path) = &args.report_html {
        write_file(path, &report_html(report))?;
    }
    Ok(())
}

fn write_file(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, body).map_err(|e| format!("could not write {}: {}", path.display(), e))
}

fn report_csv(report: &Report) -> String {
    let mut out = String::from(
        "collection,request,method,url,status,duration_ms,passed,failed,errored,extracted,misses\n",
    );
    for row in &report.results {
        let fields = [
            row.collection.clone(),
            row.request.clone(),
            row.method.clone(),
            row.url.clone(),
            row.status.clone(),
            row.duration_ms.map(|d| d.to_string()).unwrap_or_default(),
            row.passed_assertions.to_string(),
            row.failed_assertions.to_string(),
            row.errored_assertions.to_string(),
            row.extracted_count.to_string(),
            row.extractor_misses.len().to_string(),
        ];
        let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn report_html(report: &Report) -> String {
    let s = &report.summary;
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">");
    out.push_str("<title>Rusty Requester run report</title></head><body>");
    out.push_str("<h1>Run report</h1><p>");
    out.push_str(&privacy::escape_html(&format!(
        "{} request(s), {} failed. Assertions: {} pass / {} fail / {} error.",
        s.total_requests,
        s.failed_requests,
        s.passed_assertions,
        s.failed_assertions,
        s.errored_assertions
    )));
    out.push_str("</p><table border=\"1\" cellspacing=\"0\" cellpadding=\"4\">");
    out.push_str("<thead><tr><th>Collection</th><th>Request</th><th>Method</th><th>URL</th><th>Status</th><th>Duration</th><th>Assertions</th></tr></thead><tbody>");
    for row in &report.results {
        out.push_str("<tr>");
        for cell in [
            row.collection.clone(),
            row.request.clone(),
            row.method.clone(),
            row.url.clone(),
            row.status.clone(),
            row.duration_ms
                .map(|d| format!("{} ms", d))
                .unwrap_or_else(|| "-".to_string()),
            format!(
                "{} pass / {} fail / {} error",
                row.passed_assertions, row.failed_assertions, row.errored_assertions
            ),
        ] {
            out.push_str("<td>");
            out.push_str(&privacy::escape_html(&cell));
            out.push_str("</td>");
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table></body></html>");
    out
}

fn print_summary(report: &Report) {
    let s = &report.summary;
    println!(
        "{} request(s), {} failed. Assertions: {} pass / {} fail / {} error.",
        s.total_requests,
        s.failed_requests,
        s.passed_assertions,
        s.failed_assertions,
        s.errored_assertions
    );
    for row in &report.results {
        if request_failed(&row.status) || row.failed_assertions > 0 || row.errored_assertions > 0 {
            println!(
                "  FAIL {} [{}] {} {}",
                row.request, row.status, row.method, row.url
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssertionResult, Auth, HttpMethod, Request, ResponseData, SyncConfig};
    use crate::runner::{AssertionRunResult, RequestRunResult, RunIterationResult};

    fn folder(id: &str, name: &str, requests: Vec<Request>, subfolders: Vec<Folder>) -> Folder {
        Folder {
            id: id.to_string(),
            name: name.to_string(),
            requests,
            subfolders,
            description: String::new(),
            sync: SyncConfig::default(),
        }
    }

    fn request(id: &str, name: &str) -> Request {
        Request {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            method: HttpMethod::GET,
            url: "http://example.test?token=secret".to_string(),
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

    fn response(status: &str) -> ResponseData {
        ResponseData {
            body: String::new(),
            status: status.to_string(),
            time: "1 ms".to_string(),
            headers: vec![],
            set_cookies: vec![],
            response_headers_bytes: 0,
            response_body_bytes: 0,
            request_headers_bytes: 0,
            request_body_bytes: 0,
            prepare_ms: 0,
            waiting_ms: 0,
            download_ms: 0,
            total_ms: 1,
        }
    }

    fn result_with(status: &str, assertions: Vec<AssertionResult>) -> runner::CollectionRunResult {
        let requests = vec![RequestRunResult {
            request_id: "a".to_string(),
            request_name: "A".to_string(),
            method: HttpMethod::GET,
            url_template: "http://example.test?token=secret".to_string(),
            folder_path: vec!["Root".to_string()],
            response: response(status),
            assertions: assertions
                .into_iter()
                .enumerate()
                .map(|(index, result)| AssertionRunResult { index, result })
                .collect(),
            extracted: vec![],
            extractor_misses: vec![],
        }];
        let iterations = vec![RunIterationResult {
            index: 0,
            data: runner::DataRow::new(),
            requests,
        }];
        let failed = iterations[0].requests[0]
            .assertions
            .iter()
            .filter(|a| matches!(a.result, AssertionResult::Fail(_)))
            .count();
        let errored = iterations[0].requests[0]
            .assertions
            .iter()
            .filter(|a| matches!(a.result, AssertionResult::Error(_)))
            .count();
        let passed = iterations[0].requests[0].assertions.len() - failed - errored;
        runner::CollectionRunResult {
            iterations,
            total_requests: 1,
            passed_assertions: passed,
            failed_assertions: failed,
            errored_assertions: errored,
        }
    }

    #[test]
    fn resolves_folder_by_id_and_by_path() {
        let folders = vec![folder(
            "root",
            "Root",
            vec![],
            vec![folder("child", "Child", vec![request("a", "A")], vec![])],
        )];

        assert_eq!(find_folder_id(&folders, "child"), Some("child".to_string()));
        assert_eq!(
            find_folder_id(&folders, "Root/Child"),
            Some("child".to_string())
        );
        assert_eq!(find_folder_id(&folders, "Nope"), None);
    }

    #[test]
    fn resolves_request_scope_by_path_into_wrapper() {
        let folders = vec![folder(
            "root",
            "Root",
            vec![],
            vec![folder(
                "child",
                "Child",
                vec![request("a", "Login")],
                vec![],
            )],
        )];

        let scope = resolve_scope(&folders, Some("Root/Child/Login"), ScopeKind::Request).unwrap();
        match scope {
            ResolvedScope::Request(wrapper) => {
                assert_eq!(wrapper.name, "Root / Child");
                assert_eq!(wrapper.requests.len(), 1);
                assert_eq!(wrapper.requests[0].name, "Login");
            }
            _ => panic!("expected request scope"),
        }

        let by_id = resolve_scope(&folders, Some("a"), ScopeKind::Request).unwrap();
        assert!(matches!(by_id, ResolvedScope::Request(_)));

        assert!(resolve_scope(&folders, Some("missing"), ScopeKind::Request).is_err());
    }

    #[test]
    fn missing_scope_runs_everything() {
        assert!(matches!(
            resolve_scope(&[], None, ScopeKind::Folder).unwrap(),
            ResolvedScope::All
        ));
    }

    #[test]
    fn request_failed_classifies_status() {
        assert!(!request_failed("200 OK"));
        assert!(!request_failed("301 Moved Permanently"));
        assert!(request_failed("404 Not Found"));
        assert!(request_failed("500 Internal Server Error"));
        assert!(request_failed("Failed"));
    }

    #[test]
    fn exit_code_honors_fail_on_policy() {
        let clean = build_report(&result_with("200 OK", vec![AssertionResult::Pass]));
        assert_eq!(exit_code(&clean.summary, FailOn::Any), 0);

        let bad_assert = build_report(&result_with(
            "200 OK",
            vec![AssertionResult::Fail("nope".to_string())],
        ));
        assert_eq!(exit_code(&bad_assert.summary, FailOn::Any), 1);
        assert_eq!(exit_code(&bad_assert.summary, FailOn::Assertions), 1);
        assert_eq!(exit_code(&bad_assert.summary, FailOn::Requests), 0);
        assert_eq!(exit_code(&bad_assert.summary, FailOn::Never), 0);

        let bad_req = build_report(&result_with("500 Internal Server Error", vec![]));
        assert_eq!(exit_code(&bad_req.summary, FailOn::Requests), 1);
        assert_eq!(exit_code(&bad_req.summary, FailOn::Assertions), 0);
        assert_eq!(exit_code(&bad_req.summary, FailOn::Any), 1);
    }

    #[test]
    fn json_report_shape_and_redaction() {
        let report = build_report(&result_with(
            "200 OK",
            vec![AssertionResult::Fail("got 500".to_string())],
        ));
        let json = serde_json::to_string(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["summary"]["total_requests"], 1);
        assert_eq!(value["summary"]["failed_assertions"], 1);
        let row = &value["results"][0];
        assert_eq!(row["status"], "200 OK");
        assert_eq!(row["method"], "GET");
        assert_eq!(row["assertions"][0]["outcome"], "fail");
        let url = row["url"].as_str().unwrap();
        assert!(!url.contains("secret"), "url must be redacted: {}", url);
    }

    #[test]
    fn csv_report_quotes_and_lists_rows() {
        let report = build_report(&result_with("200 OK", vec![AssertionResult::Pass]));
        let csv = report_csv(&report);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("collection,request,method,url,status"));
        assert!(!csv.contains("secret"));
    }

    #[test]
    fn html_report_escapes_and_renders_table() {
        let report = build_report(&result_with("200 OK", vec![AssertionResult::Pass]));
        let html = report_html(&report);
        assert!(html.contains("<table"));
        assert!(html.contains("200 OK"));
        assert!(!html.contains("secret"));
    }

    #[test]
    fn parse_args_reads_options() {
        let args: Vec<String> = [
            "API/Auth",
            "--kind",
            "folder",
            "--env",
            "Staging",
            "--fail-on",
            "requests",
            "--report-json",
            "out.json",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.scope.as_deref(), Some("API/Auth"));
        assert_eq!(parsed.kind, ScopeKind::Folder);
        assert_eq!(parsed.env.as_deref(), Some("Staging"));
        assert_eq!(parsed.fail_on, FailOn::Requests);
        assert_eq!(parsed.report_json, Some(PathBuf::from("out.json")));

        assert!(parse_args(&["--bogus".to_string()]).is_err());
        assert!(parse_args(&["--kind".to_string()]).is_err());
    }
}
