/// Basic robots.txt parser — checks Disallow/Allow rules for a user-agent.
#[derive(Debug, Clone)]
pub struct RobotsChecker {
    rules: Vec<RobotsRule>,
}

#[derive(Debug, Clone)]
struct RobotsRule {
    user_agent: String,
    disallow: Vec<String>,
    allow: Vec<String>,
}

impl RobotsChecker {
    /// Parse a robots.txt body into a checker.
    pub fn parse(body: &str) -> Self {
        let mut rules: Vec<RobotsRule> = Vec::new();
        let mut current_ua: Option<String> = None;
        let mut current_disallow: Vec<String> = Vec::new();
        let mut current_allow: Vec<String> = Vec::new();

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("User-agent:").or_else(|| line.strip_prefix("user-agent:")) {
                // Flush previous rule
                if let Some(ua) = current_ua.take() {
                    rules.push(RobotsRule {
                        user_agent: ua,
                        disallow: std::mem::take(&mut current_disallow),
                        allow: std::mem::take(&mut current_allow),
                    });
                }
                current_ua = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("Disallow:").or_else(|| line.strip_prefix("disallow:")) {
                let path = rest.trim().to_string();
                if !path.is_empty() {
                    current_disallow.push(path);
                }
            } else if let Some(rest) = line.strip_prefix("Allow:").or_else(|| line.strip_prefix("allow:")) {
                let path = rest.trim().to_string();
                if !path.is_empty() {
                    current_allow.push(path);
                }
            }
        }

        // Flush last rule
        if let Some(ua) = current_ua {
            rules.push(RobotsRule {
                user_agent: ua,
                disallow: current_disallow,
                allow: current_allow,
            });
        }

        Self { rules }
    }

    /// Check whether a path is allowed for the given user-agent.
    /// Returns true if the path is allowed (not disallowed).
    pub fn is_allowed(&self, user_agent: &str, path: &str) -> bool {
        // Find the most specific matching rule set
        let matching_rules: Vec<&RobotsRule> = self
            .rules
            .iter()
            .filter(|r| r.user_agent == "*" || user_agent.contains(&r.user_agent))
            .collect();

        if matching_rules.is_empty() {
            return true; // No rules = allowed
        }

        for rule in &matching_rules {
            // Allow takes precedence over Disallow for same-length prefix
            for allow_path in &rule.allow {
                if path.starts_with(allow_path.as_str()) {
                    return true;
                }
            }
            for disallow_path in &rule.disallow {
                if path.starts_with(disallow_path.as_str()) {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let txt = "User-agent: *\nDisallow: /private\nDisallow: /admin\n";
        let checker = RobotsChecker::parse(txt);
        assert!(!checker.is_allowed("vil-crawler", "/private/page"));
        assert!(!checker.is_allowed("vil-crawler", "/admin"));
        assert!(checker.is_allowed("vil-crawler", "/public"));
    }

    #[test]
    fn test_allow_override() {
        let txt = "User-agent: *\nDisallow: /api\nAllow: /api/public\n";
        let checker = RobotsChecker::parse(txt);
        assert!(checker.is_allowed("vil-crawler", "/api/public/data"));
        assert!(!checker.is_allowed("vil-crawler", "/api/private"));
    }

    #[test]
    fn test_empty_robots() {
        let checker = RobotsChecker::parse("");
        assert!(checker.is_allowed("vil-crawler", "/anything"));
    }

    #[test]
    fn test_specific_user_agent() {
        let txt = "User-agent: Googlebot\nDisallow: /secret\n\nUser-agent: *\nDisallow: /\n";
        let checker = RobotsChecker::parse(txt);
        // Wildcard rule disallows everything
        assert!(!checker.is_allowed("vil-crawler", "/page"));
    }
}
