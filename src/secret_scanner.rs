use crate::model::{Auth, BodyExt, Folder, KvRow, Request};
use crate::privacy::{is_sensitive_key, mask_secret_value, redact_url_query_and_fragment};
use serde_json::Value;

pub const REDACTED: &str = "[REDACTED]";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretFinding {
    pub path: String,
    pub kind: SecretKind,
    pub evidence: String,
}

impl SecretFinding {
    pub fn summary(&self) -> String {
        format!("{}: {} ({})", self.path, self.kind.label(), self.evidence)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    SensitiveKeyName,
    AwsAccessKey,
    GithubToken,
    GoogleApiKey,
    Jwt,
    SlackToken,
    StripeKey,
    SecretAssignment,
}

impl SecretKind {
    fn label(self) -> &'static str {
        match self {
            SecretKind::SensitiveKeyName => "sensitive key",
            SecretKind::AwsAccessKey => "AWS access key",
            SecretKind::GithubToken => "GitHub token",
            SecretKind::GoogleApiKey => "Google API key",
            SecretKind::Jwt => "JWT-like token",
            SecretKind::SlackToken => "Slack token",
            SecretKind::StripeKey => "Stripe key",
            SecretKind::SecretAssignment => "secret assignment",
        }
    }
}

pub fn scan_folders(folders: &[Folder]) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    for folder in folders {
        scan_folder(folder, &folder.name, &mut findings);
    }
    findings
}

pub fn redact_folders(folders: &[Folder]) -> Vec<Folder> {
    folders.iter().map(redact_folder).collect()
}

fn scan_folder(folder: &Folder, path: &str, findings: &mut Vec<SecretFinding>) {
    scan_text_patterns(
        &folder.description,
        &format!("{} description", path),
        findings,
    );

    for request in &folder.requests {
        scan_request(request, &format!("{} / {}", path, request.name), findings);
    }
    for child in &folder.subfolders {
        scan_folder(child, &format!("{} / {}", path, child.name), findings);
    }
}

fn scan_request(request: &Request, path: &str, findings: &mut Vec<SecretFinding>) {
    scan_url(&request.url, &format!("{} url", path), findings);
    scan_rows(&request.query_params, &format!("{} query", path), findings);
    scan_rows(
        &request.path_params,
        &format!("{} path params", path),
        findings,
    );
    scan_rows(&request.headers, &format!("{} headers", path), findings);
    scan_rows(&request.cookies, &format!("{} cookies", path), findings);
    scan_body(&request.body, &format!("{} body", path), findings);

    if let Some(body_ext) = &request.body_ext {
        match body_ext {
            BodyExt::FormUrlEncoded { fields } => {
                scan_rows(fields, &format!("{} form body", path), findings);
            }
            BodyExt::MultipartForm { fields } => {
                scan_rows(fields, &format!("{} multipart body", path), findings);
            }
            BodyExt::GraphQL { variables } => {
                scan_body(variables, &format!("{} GraphQL variables", path), findings);
            }
        }
    }

    match &request.auth {
        Auth::None => {}
        Auth::Bearer { token } => scan_named_secret(
            "token",
            token,
            &format!("{} auth bearer token", path),
            findings,
        ),
        Auth::Basic { password, .. } => scan_named_secret(
            "password",
            password,
            &format!("{} auth basic password", path),
            findings,
        ),
        Auth::OAuth2(state) => {
            scan_named_secret(
                "client_secret",
                &state.config.client_secret,
                &format!("{} OAuth client secret", path),
                findings,
            );
            scan_named_secret(
                "access_token",
                &state.access_token,
                &format!("{} OAuth access token", path),
                findings,
            );
            scan_named_secret(
                "refresh_token",
                &state.refresh_token,
                &format!("{} OAuth refresh token", path),
                findings,
            );
        }
    }
}

fn scan_url(url: &str, path: &str, findings: &mut Vec<SecretFinding>) {
    scan_text_patterns(url, path, findings);

    for part in url.split(&['?', '#'][..]).skip(1) {
        for pair in part.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            scan_named_secret(key, value, path, findings);
        }
    }
}

fn scan_rows(rows: &[KvRow], path: &str, findings: &mut Vec<SecretFinding>) {
    for row in rows {
        let row_path = if row.key.trim().is_empty() {
            path.to_string()
        } else {
            format!("{}.{}", path, row.key.trim())
        };
        scan_named_secret(&row.key, &row.value, &row_path, findings);
        scan_text_patterns(
            &row.description,
            &format!("{} description", row_path),
            findings,
        );
    }
}

fn scan_named_secret(key: &str, value: &str, path: &str, findings: &mut Vec<SecretFinding>) {
    if is_sensitive_key(key) && has_exported_secret_value(value) {
        findings.push(SecretFinding {
            path: path.to_string(),
            kind: SecretKind::SensitiveKeyName,
            evidence: mask_secret_value(value.trim()),
        });
    }
    scan_text_patterns(value, path, findings);
}

fn scan_body(body: &str, path: &str, findings: &mut Vec<SecretFinding>) {
    if body.trim().is_empty() {
        return;
    }

    if let Ok(json) = serde_json::from_str::<Value>(body) {
        scan_json_value(&json, path, findings);
    } else {
        scan_sensitive_assignments(body, path, findings);
        scan_text_patterns(body, path, findings);
    }
}

fn scan_json_value(value: &Value, path: &str, findings: &mut Vec<SecretFinding>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };
                if is_sensitive_key(key) && json_value_has_secret_material(value) {
                    findings.push(SecretFinding {
                        path: child_path.clone(),
                        kind: SecretKind::SensitiveKeyName,
                        evidence: json_evidence(value),
                    });
                }
                scan_json_value(value, &child_path, findings);
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                scan_json_value(item, &format!("{}[{}]", path, idx), findings);
            }
        }
        Value::String(text) => scan_text_patterns(text, path, findings),
        _ => {}
    }
}

fn scan_text_patterns(text: &str, path: &str, findings: &mut Vec<SecretFinding>) {
    for candidate in secret_candidates(text) {
        if let Some(kind) = classify_secret_token(candidate) {
            findings.push(SecretFinding {
                path: path.to_string(),
                kind,
                evidence: mask_secret_value(candidate),
            });
        }
    }
}

fn scan_sensitive_assignments(text: &str, path: &str, findings: &mut Vec<SecretFinding>) {
    for chunk in text.split(&['\n', '\r', '&', ',', '{', '}', ';'][..]) {
        let Some((key, value)) = chunk.split_once('=').or_else(|| chunk.split_once(':')) else {
            continue;
        };
        let key = key.trim().trim_matches(&['"', '\'', ' ', '\t'][..]);
        let value = value.trim().trim_matches(&['"', '\'', ' ', '\t'][..]);
        if is_sensitive_key(key) && has_exported_secret_value(value) {
            findings.push(SecretFinding {
                path: path.to_string(),
                kind: SecretKind::SecretAssignment,
                evidence: format!("{}={}", key, mask_secret_value(value)),
            });
        }
    }
}

fn secret_candidates(text: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    let mut start = None;

    for (idx, ch) in text.char_indices() {
        if is_secret_char(ch) {
            start.get_or_insert(idx);
        } else if let Some(begin) = start.take() {
            push_candidate(&mut candidates, &text[begin..idx]);
        }
    }
    if let Some(begin) = start {
        push_candidate(&mut candidates, &text[begin..]);
    }

    candidates
}

fn push_candidate<'a>(candidates: &mut Vec<&'a str>, token: &'a str) {
    let token = token.trim_matches(&['.', '-', '_'][..]);
    if token.len() >= 12 {
        candidates.push(token);
    }
}

fn is_secret_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

fn classify_secret_token(token: &str) -> Option<SecretKind> {
    if is_aws_access_key(token) {
        Some(SecretKind::AwsAccessKey)
    } else if is_github_token(token) {
        Some(SecretKind::GithubToken)
    } else if is_google_api_key(token) {
        Some(SecretKind::GoogleApiKey)
    } else if is_slack_token(token) {
        Some(SecretKind::SlackToken)
    } else if is_stripe_key(token) {
        Some(SecretKind::StripeKey)
    } else if is_jwt_like(token) {
        Some(SecretKind::Jwt)
    } else {
        None
    }
}

fn is_aws_access_key(token: &str) -> bool {
    token.len() == 20
        && (token.starts_with("AKIA") || token.starts_with("ASIA"))
        && token
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn is_github_token(token: &str) -> bool {
    (matches!(
        token.get(..4),
        Some("ghp_") | Some("gho_") | Some("ghu_") | Some("ghs_") | Some("ghr_")
    ) && token.len() >= 30)
        || (token.starts_with("github_pat_") && token.len() >= 40)
}

fn is_google_api_key(token: &str) -> bool {
    token.starts_with("AIza")
        && token.len() >= 35
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn is_slack_token(token: &str) -> bool {
    (token.starts_with("xoxb-")
        || token.starts_with("xoxa-")
        || token.starts_with("xoxp-")
        || token.starts_with("xoxr-")
        || token.starts_with("xoxs-"))
        && token.matches('-').count() >= 2
        && token.len() >= 20
}

fn is_stripe_key(token: &str) -> bool {
    (token.starts_with("sk_live_")
        || token.starts_with("sk_test_")
        || token.starts_with("rk_live_")
        || token.starts_with("rk_test_"))
        && token.len() >= 20
}

fn is_jwt_like(token: &str) -> bool {
    let parts = token.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts[0].starts_with("eyJ")
        && parts.iter().all(|part| {
            part.len() >= 8
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        })
}

fn has_exported_secret_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !is_placeholder_or_redacted(value)
}

fn is_placeholder_or_redacted(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    (lower.starts_with("{{") && lower.ends_with("}}"))
        || matches!(
            lower.as_str(),
            "[redacted]" | "redacted" | "<redacted>" | "<password>" | "<token>" | "<secret>"
        )
}

fn json_value_has_secret_material(value: &Value) -> bool {
    match value {
        Value::String(text) => has_exported_secret_value(text),
        Value::Array(items) => items.iter().any(json_value_has_secret_material),
        Value::Object(map) => map.values().any(json_value_has_secret_material),
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn json_evidence(value: &Value) -> String {
    match value {
        Value::String(text) => mask_secret_value(text),
        Value::Array(_) => "array value".to_string(),
        Value::Object(_) => "object value".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(_) | Value::Number(_) => "scalar value".to_string(),
    }
}

fn redact_folder(folder: &Folder) -> Folder {
    Folder {
        id: folder.id.clone(),
        name: folder.name.clone(),
        requests: folder.requests.iter().map(redact_request).collect(),
        subfolders: folder.subfolders.iter().map(redact_folder).collect(),
        description: redact_text(&folder.description),
    }
}

fn redact_request(request: &Request) -> Request {
    Request {
        id: request.id.clone(),
        name: request.name.clone(),
        description: redact_text(&request.description),
        method: request.method.clone(),
        url: redact_url(&request.url),
        query_params: redact_rows(&request.query_params),
        path_params: redact_rows(&request.path_params),
        headers: redact_rows(&request.headers),
        cookies: redact_rows(&request.cookies),
        body: redact_body(&request.body),
        body_ext: request.body_ext.as_ref().map(redact_body_ext),
        auth: redact_auth(&request.auth),
        extractors: request.extractors.clone(),
        assertions: request.assertions.clone(),
        source: request.source.clone(),
    }
}

fn redact_rows(rows: &[KvRow]) -> Vec<KvRow> {
    rows.iter()
        .map(|row| KvRow {
            enabled: row.enabled,
            key: row.key.clone(),
            value: if is_sensitive_key(&row.key) && has_exported_secret_value(&row.value) {
                REDACTED.to_string()
            } else {
                redact_text(&row.value)
            },
            description: redact_text(&row.description),
        })
        .collect()
}

fn redact_auth(auth: &Auth) -> Auth {
    match auth {
        Auth::None => Auth::None,
        Auth::Bearer { token } => Auth::Bearer {
            token: redact_named_value("token", token),
        },
        Auth::Basic { username, password } => Auth::Basic {
            username: username.clone(),
            password: redact_named_value("password", password),
        },
        Auth::OAuth2(state) => {
            let mut state = (**state).clone();
            state.config.client_secret =
                redact_named_value("client_secret", &state.config.client_secret);
            state.access_token = redact_named_value("access_token", &state.access_token);
            state.refresh_token = redact_named_value("refresh_token", &state.refresh_token);
            Auth::OAuth2(Box::new(state))
        }
    }
}

fn redact_body_ext(body_ext: &BodyExt) -> BodyExt {
    match body_ext {
        BodyExt::FormUrlEncoded { fields } => BodyExt::FormUrlEncoded {
            fields: redact_rows(fields),
        },
        BodyExt::MultipartForm { fields } => BodyExt::MultipartForm {
            fields: redact_rows(fields),
        },
        BodyExt::GraphQL { variables } => BodyExt::GraphQL {
            variables: redact_body(variables),
        },
    }
}

fn redact_named_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) && has_exported_secret_value(value) {
        REDACTED.to_string()
    } else {
        redact_text(value)
    }
}

fn redact_url(url: &str) -> String {
    let mut should_redact_url = false;
    let mut findings = Vec::new();
    scan_url(url, "url", &mut findings);
    if !findings.is_empty() {
        should_redact_url = true;
    }

    if should_redact_url {
        redact_url_query_and_fragment(url)
    } else {
        url.to_string()
    }
}

fn redact_body(body: &str) -> String {
    if body.trim().is_empty() {
        return String::new();
    }

    if let Ok(mut json) = serde_json::from_str::<Value>(body) {
        redact_json_value(&mut json, false);
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| redact_text(body))
    } else {
        redact_assignments(&redact_text(body))
    }
}

fn redact_json_value(value: &mut Value, force: bool) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                redact_json_value(value, force || is_sensitive_key(key));
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json_value(item, force);
            }
        }
        Value::String(text) => {
            if force && has_exported_secret_value(text) {
                *text = REDACTED.to_string();
            } else {
                *text = redact_text(text);
            }
        }
        Value::Number(_) | Value::Bool(_) => {
            if force {
                *value = Value::String(REDACTED.to_string());
            }
        }
        Value::Null => {}
    }
}

fn redact_text(text: &str) -> String {
    let mut out = text.to_string();
    for candidate in secret_candidates(text) {
        if classify_secret_token(candidate).is_some() {
            out = out.replace(candidate, REDACTED);
        }
    }
    out
}

fn redact_assignments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if let Some(redacted) = redact_assignment_line(line) {
            out.push_str(&redacted);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !text.ends_with('\n') {
        out.pop();
    }
    out
}

fn redact_assignment_line(line: &str) -> Option<String> {
    let (delimiter, idx) = line
        .find('=')
        .map(|idx| ('=', idx))
        .or_else(|| line.find(':').map(|idx| (':', idx)))?;
    let key = line[..idx].trim().trim_matches(&['"', '\'', ' ', '\t'][..]);
    if !is_sensitive_key(key) {
        return None;
    }
    let value = line[idx + delimiter.len_utf8()..].trim();
    if !has_exported_secret_value(value.trim_matches(&['"', '\''][..])) {
        return None;
    }
    Some(format!("{}{} {}", &line[..idx], delimiter, REDACTED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HttpMethod, OAuth2Config, OAuth2State};

    fn request_with_secret(key: &str, value: &str) -> Request {
        Request {
            id: "r".into(),
            name: "Get user".into(),
            description: String::new(),
            method: HttpMethod::GET,
            url: "https://api.example.com/users".into(),
            query_params: vec![KvRow::new("page", "1")],
            path_params: vec![],
            headers: vec![KvRow::new(key, value)],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
            assertions: vec![],
            source: None,
        }
    }

    fn folder_with_request(request: Request) -> Vec<Folder> {
        vec![Folder {
            id: "f".into(),
            name: "API".into(),
            requests: vec![request],
            subfolders: vec![],
            description: String::new(),
        }]
    }

    #[test]
    fn detects_sensitive_key_names() {
        let folders = folder_with_request(request_with_secret("X-API-Key", "secret-value"));
        let findings = scan_folders(&folders);

        assert!(findings
            .iter()
            .any(|finding| finding.kind == SecretKind::SensitiveKeyName));
    }

    #[test]
    fn detects_common_token_patterns() {
        let mut request =
            request_with_secret("X-Trace", "ghp_1234567890abcdefghijklmnopqrstuvwxyz");
        request.body =
            r#"{"aws":"AKIA1234567890ABCDEF","stripe":"sk_live_1234567890abcdef"}"#.into();
        request.auth = Auth::OAuth2(Box::new(OAuth2State {
            config: OAuth2Config {
                client_secret: "AIza1234567890abcdefghijklmnopqrstuv".into(),
                ..Default::default()
            },
            access_token: "xoxb-1234567890-abcdefghi".into(),
            refresh_token: "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature1".into(),
            expires_at: None,
        }));

        let findings = scan_folders(&folder_with_request(request));
        for kind in [
            SecretKind::GithubToken,
            SecretKind::AwsAccessKey,
            SecretKind::StripeKey,
            SecretKind::GoogleApiKey,
            SecretKind::SlackToken,
            SecretKind::Jwt,
        ] {
            assert!(
                findings.iter().any(|finding| finding.kind == kind),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn masks_redacted_export_values() {
        let mut request = request_with_secret("Authorization", "Bearer secret-token");
        request.body = r#"{"password":"secret","nested":{"token":"abc123"}}"#.into();
        request.cookies = vec![KvRow::new("session", "cookie-secret")];
        request.auth = Auth::Basic {
            username: "alice".into(),
            password: "password123".into(),
        };

        let redacted = redact_folders(&folder_with_request(request));
        let request = &redacted[0].requests[0];
        assert_eq!(request.headers[0].value, REDACTED);
        assert_eq!(request.cookies[0].value, REDACTED);
        assert!(matches!(&request.auth, Auth::Basic { password, .. } if password == REDACTED));
        assert!(request.body.contains(REDACTED));
        assert!(!request.body.contains("secret"));
        assert!(!request.body.contains("abc123"));
    }

    #[test]
    fn leaves_false_positive_safe_cases_alone() {
        let mut request = request_with_secret("Content-Type", "application/json");
        request.query_params = vec![KvRow::new("token", "{{token}}")];
        request.body =
            r#"{"monkey":"banana","access_token":"{{token}}","note":"sketch_live_demo"}"#.into();

        let folders = folder_with_request(request.clone());
        assert!(scan_folders(&folders).is_empty());

        let redacted = redact_folders(&folders);
        assert_eq!(redacted[0].requests[0].query_params[0].value, "{{token}}");
        assert!(redacted[0].requests[0].body.contains("sketch_live_demo"));
    }
}
