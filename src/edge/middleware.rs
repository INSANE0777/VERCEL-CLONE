use serde::{Deserialize, Serialize};

/// Edge middleware rule types.
/// These compile into native Caddy directives — no separate middleware process needed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MiddlewareType {
    /// HTTP redirect (301/302/307/308)
    Redirect,
    /// URL rewrite (internal, URL stays the same in browser)
    Rewrite,
    /// Custom response header per path pattern
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareRule {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub rule_type: MiddlewareType,
    /// Path pattern to match (e.g. "/old-path" or "/api/*")
    pub pattern: String,
    /// Redirect destination, rewrite target, or header value
    pub target: String,
    /// For redirects: status code (default 301). For headers: header name.
    pub status_code: Option<i32>,
    /// For headers: the header name to set
    pub header_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMiddlewareRuleRequest {
    pub rule_type: MiddlewareType,
    pub pattern: String,
    pub target: String,
    pub status_code: Option<i32>,
    pub header_name: Option<String>,
}

/// Compile middleware rules into Caddy config directives.
/// Returns a string fragment to embed inside a Caddy site block.
pub fn compile_middleware(rules: &[MiddlewareRule]) -> String {
    let mut directives = String::new();

    for rule in rules {
        match rule.rule_type {
            MiddlewareType::Redirect => {
                let status = rule.status_code.unwrap_or(301);
                // Caddy: redir <matcher> <to> <status>
                if rule.pattern == "/*" || rule.pattern == "/" {
                    directives.push_str(&format!(
                        "    redir * {} {}\n",
                        rule.target, status
                    ));
                } else {
                    let matcher = path_to_caddy_matcher(&rule.pattern);
                    directives.push_str(&format!(
                        "    redir {} {} {}\n",
                        matcher, rule.target, status
                    ));
                }
            }
            MiddlewareType::Rewrite => {
                // Caddy: rewrite <matcher> <to>
                if rule.pattern == "/*" || rule.pattern == "/" {
                    directives.push_str(&format!(
                        "    rewrite * {}\n",
                        rule.target
                    ));
                } else {
                    let matcher = path_to_caddy_matcher(&rule.pattern);
                    directives.push_str(&format!(
                        "    rewrite {} {}\n",
                        matcher, rule.target
                    ));
                }
            }
            MiddlewareType::Header => {
                let name = rule.header_name.as_deref().unwrap_or("X-Custom");
                let matcher = path_to_caddy_matcher(&rule.pattern);
                directives.push_str(&format!(
                    "    header {} {} \"{}\"\n",
                    matcher, name, rule.target
                ));
            }
        }
    }

    directives
}

/// Convert a path pattern like "/api/*" into a Caddy matcher like "@api path /api/*"
fn path_to_caddy_matcher(pattern: &str) -> String {
    if pattern.contains('*') || pattern.contains('{') {
        // Named matcher for complex patterns
        let name = pattern.trim_start_matches('/').replace('*', "wildcard").replace(['/', '{', '}'], "_");
        format!("@{} path {}", name, pattern)
    } else {
        // Simple path matcher
        format!("@path_{} path {}", pattern.trim_start_matches('/').replace(['/', '.'], "_"), pattern)
    }
}

impl MiddlewareType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MiddlewareType::Redirect => "redirect",
            MiddlewareType::Rewrite => "rewrite",
            MiddlewareType::Header => "header",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "redirect" => MiddlewareType::Redirect,
            "rewrite" => MiddlewareType::Rewrite,
            _ => MiddlewareType::Header,
        }
    }

    pub fn to_db(&self) -> &'static str {
        self.as_str()
    }
}
