use crate::curl;
use crate::model::{Auth, Request};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SnippetLang {
    Curl,
    Python,
    JavaScript,
    HttpieShell,
}

impl SnippetLang {
    pub fn label(&self) -> &'static str {
        match self {
            SnippetLang::Curl => "cURL",
            SnippetLang::Python => "Python (requests)",
            SnippetLang::JavaScript => "JavaScript (fetch)",
            SnippetLang::HttpieShell => "HTTPie",
        }
    }
}

pub fn render_snippet(req: &Request, lang: SnippetLang) -> String {
    match lang {
        SnippetLang::Curl => curl::to_curl(req),
        SnippetLang::Python => python(req),
        SnippetLang::JavaScript => javascript(req),
        SnippetLang::HttpieShell => httpie(req),
    }
}

fn collect_send_headers(req: &Request) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    if let Auth::Bearer { token } = &req.auth {
        if !token.is_empty() {
            out.push(("Authorization".into(), format!("Bearer {}", token)));
        }
    }
    out
}

fn python(req: &Request) -> String {
    let mut s = String::new();
    s.push_str("import requests\n\n");
    let url = curl::build_full_url(&req.url, &req.query_params);
    s.push_str(&format!("url = {:?}\n", url));
    let headers = collect_send_headers(req);
    if !headers.is_empty() {
        s.push_str("headers = {\n");
        for (k, v) in &headers {
            s.push_str(&format!("    {:?}: {:?},\n", k, v));
        }
        s.push_str("}\n");
    } else {
        s.push_str("headers = {}\n");
    }
    let cookies: Vec<(&String, &String)> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| (&c.key, &c.value))
        .collect();
    if !cookies.is_empty() {
        s.push_str("cookies = {\n");
        for (k, v) in &cookies {
            s.push_str(&format!("    {:?}: {:?},\n", k, v));
        }
        s.push_str("}\n");
    }
    let mut auth_arg = String::new();
    if let Auth::Basic { username, password } = &req.auth {
        if !username.is_empty() {
            auth_arg = format!(", auth=({:?}, {:?})", username, password);
        }
    }
    let cookies_arg = if cookies.is_empty() { "" } else { ", cookies=cookies" };
    let method = format!("{}", req.method).to_lowercase();
    if !req.body.is_empty() {
        s.push_str(&format!("payload = {:?}\n\n", req.body));
        s.push_str(&format!(
            "response = requests.{}(url, headers=headers{}{}, data=payload)\n",
            method, cookies_arg, auth_arg
        ));
    } else {
        s.push_str(&format!(
            "\nresponse = requests.{}(url, headers=headers{}{})\n",
            method, cookies_arg, auth_arg
        ));
    }
    s.push_str("print(response.status_code)\nprint(response.text)\n");
    s
}

fn javascript(req: &Request) -> String {
    let mut s = String::new();
    let url = curl::build_full_url(&req.url, &req.query_params);
    s.push_str(&format!("const url = {:?};\n\n", url));
    s.push_str("const options = {\n");
    s.push_str(&format!("  method: {:?},\n", req.method.to_string()));
    let headers = collect_send_headers(req);
    let cookies: Vec<String> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| format!("{}={}", c.key, c.value))
        .collect();
    let mut header_lines: Vec<String> = headers
        .iter()
        .map(|(k, v)| format!("    {:?}: {:?}", k, v))
        .collect();
    if !cookies.is_empty() {
        header_lines.push(format!("    \"Cookie\": {:?}", cookies.join("; ")));
    }
    if !header_lines.is_empty() {
        s.push_str("  headers: {\n");
        s.push_str(&header_lines.join(",\n"));
        s.push_str(",\n  },\n");
    }
    if !req.body.is_empty() {
        s.push_str(&format!("  body: {:?},\n", req.body));
    }
    s.push_str("};\n\n");
    s.push_str("fetch(url, options)\n");
    s.push_str("  .then((res) => res.text())\n");
    s.push_str("  .then(console.log)\n");
    s.push_str("  .catch(console.error);\n");
    s
}

fn httpie(req: &Request) -> String {
    let mut parts: Vec<String> = vec!["http".into(), format!("{}", req.method)];
    let url = curl::build_full_url(&req.url, &req.query_params);
    parts.push(format!("'{}'", url.replace('\'', "'\\''")));
    for (k, v) in collect_send_headers(req) {
        parts.push(format!("'{}:{}'", k, v));
    }
    if let Auth::Basic { username, password } = &req.auth {
        if !username.is_empty() {
            parts.push(format!("--auth='{}:{}'", username, password));
        }
    }
    let cookies: Vec<String> = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| format!("{}={}", c.key, c.value))
        .collect();
    if !cookies.is_empty() {
        parts.push(format!("'Cookie:{}'", cookies.join("; ")));
    }
    let mut s = parts.join(" ");
    if !req.body.is_empty() {
        s.push_str(&format!(
            "\n# body (use --raw or pipe stdin):\n# echo {:?} | http ...",
            req.body
        ));
    }
    s
}
