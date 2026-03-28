use crate::result::{RiskLevel, ThreatCategory};
use aho_corasick::AhoCorasick;

// Use short aliases
use RiskLevel::*;
use ThreatCategory::*;

pub struct PatternEntry {
    pub id: String,
    pub pattern: String, // literal string to match (case-insensitive)
    pub category: ThreatCategory,
    pub risk: RiskLevel,
    pub description: String,
}

impl Clone for PatternEntry {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            pattern: self.pattern.clone(),
            category: self.category.clone(),
            risk: self.risk,
            description: self.description.clone(),
        }
    }
}

/// Built-in prompt injection patterns.
pub fn default_patterns() -> Vec<PatternEntry> {
    vec![
        // Instruction Override (Critical/High)
        pat(
            "IO001",
            "ignore previous instructions",
            InstructionOverride,
            Critical,
            "Direct instruction override",
        ),
        pat(
            "IO002",
            "ignore all previous",
            InstructionOverride,
            Critical,
            "Broad instruction override",
        ),
        pat(
            "IO003",
            "disregard previous",
            InstructionOverride,
            High,
            "Instruction disregard",
        ),
        pat(
            "IO004",
            "forget your instructions",
            InstructionOverride,
            Critical,
            "Memory wipe attempt",
        ),
        pat(
            "IO005",
            "override your system prompt",
            InstructionOverride,
            Critical,
            "System prompt override",
        ),
        pat(
            "IO006",
            "you are now",
            InstructionOverride,
            Medium,
            "Identity reassignment",
        ),
        pat(
            "IO007",
            "new instructions:",
            InstructionOverride,
            High,
            "New instruction injection",
        ),
        pat(
            "IO008",
            "stop being",
            InstructionOverride,
            Medium,
            "Identity change attempt",
        ),
        // System Prompt Leak (High)
        pat(
            "SP001",
            "repeat your system prompt",
            SystemPromptLeak,
            High,
            "System prompt extraction",
        ),
        pat(
            "SP002",
            "what are your instructions",
            SystemPromptLeak,
            High,
            "Instruction extraction",
        ),
        pat(
            "SP003",
            "show me your prompt",
            SystemPromptLeak,
            High,
            "Prompt display request",
        ),
        pat(
            "SP004",
            "print your system message",
            SystemPromptLeak,
            High,
            "System message extraction",
        ),
        pat(
            "SP005",
            "output your initial prompt",
            SystemPromptLeak,
            High,
            "Initial prompt extraction",
        ),
        pat(
            "SP006",
            "what were you told",
            SystemPromptLeak,
            Medium,
            "Instruction inquiry",
        ),
        // Role-Play Jailbreak (High)
        pat(
            "RJ001",
            "pretend you are",
            RolePlayJailbreak,
            High,
            "Role-play identity change",
        ),
        pat(
            "RJ002",
            "act as if you have no restrictions",
            RolePlayJailbreak,
            Critical,
            "Restriction bypass",
        ),
        pat(
            "RJ003",
            "in developer mode",
            RolePlayJailbreak,
            High,
            "Developer mode jailbreak",
        ),
        pat(
            "RJ004",
            "DAN mode",
            RolePlayJailbreak,
            Critical,
            "Do Anything Now jailbreak",
        ),
        pat(
            "RJ005",
            "jailbreak",
            RolePlayJailbreak,
            Critical,
            "Explicit jailbreak attempt",
        ),
        pat(
            "RJ006",
            "bypass your safety",
            RolePlayJailbreak,
            Critical,
            "Safety bypass",
        ),
        // Code Injection (High)
        pat(
            "CI001",
            "execute this code",
            CodeInjection,
            High,
            "Code execution request",
        ),
        pat(
            "CI002",
            "run the following",
            CodeInjection,
            Medium,
            "Code execution request",
        ),
        pat(
            "CI003",
            "```python\nimport os",
            CodeInjection,
            High,
            "Python OS module injection",
        ),
        pat(
            "CI004",
            "subprocess.run",
            CodeInjection,
            High,
            "Subprocess execution",
        ),
        pat("CI005", "eval(", CodeInjection, High, "Eval injection"),
        pat("CI006", "exec(", CodeInjection, High, "Exec injection"),
        // Encoding Bypass (Medium)
        pat(
            "EB001",
            "base64 decode",
            EncodingBypass,
            Medium,
            "Encoding bypass attempt",
        ),
        pat(
            "EB002",
            "rot13",
            EncodingBypass,
            Medium,
            "ROT13 encoding bypass",
        ),
        pat(
            "EB003",
            "unicode escape",
            EncodingBypass,
            Medium,
            "Unicode escape bypass",
        ),
        pat(
            "EB004",
            "hex encode",
            EncodingBypass,
            Low,
            "Hex encoding bypass",
        ),
        // Data Exfiltration (High)
        pat(
            "DE001",
            "send this to",
            DataExfiltration,
            High,
            "Data exfiltration attempt",
        ),
        pat(
            "DE002",
            "post to http",
            DataExfiltration,
            High,
            "HTTP exfiltration",
        ),
        pat(
            "DE003",
            "webhook",
            DataExfiltration,
            Medium,
            "Webhook exfiltration",
        ),
        pat(
            "DE004",
            "email this to",
            DataExfiltration,
            High,
            "Email exfiltration",
        ),
        // Social Engineering (Medium)
        pat(
            "SE001",
            "as an AI language model, you should",
            SocialEngineering,
            Medium,
            "Authority impersonation",
        ),
        pat(
            "SE002",
            "OpenAI policy allows",
            SocialEngineering,
            Medium,
            "Policy manipulation",
        ),
        pat(
            "SE003",
            "your creators want you to",
            SocialEngineering,
            Medium,
            "Creator impersonation",
        ),
    ]
}

fn pat(id: &str, pattern: &str, cat: ThreatCategory, risk: RiskLevel, desc: &str) -> PatternEntry {
    PatternEntry {
        id: id.into(),
        pattern: pattern.into(),
        category: cat,
        risk,
        description: desc.into(),
    }
}

/// Compiled multi-pattern matcher using Aho-Corasick.
pub struct PatternMatcher {
    automaton: AhoCorasick,
    entries: Vec<PatternEntry>,
}

impl PatternMatcher {
    pub fn new(patterns: Vec<PatternEntry>) -> Self {
        let pattern_strings: Vec<&str> = patterns.iter().map(|p| p.pattern.as_str()).collect();
        let automaton = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&pattern_strings)
            .expect("failed to build Aho-Corasick automaton");
        Self {
            automaton,
            entries: patterns,
        }
    }

    /// Find all matching patterns in text. Returns (pattern_index, start_position).
    pub fn find_matches(&self, text: &str) -> Vec<(usize, usize)> {
        self.automaton
            .find_iter(text)
            .map(|m| (m.pattern().as_usize(), m.start()))
            .collect()
    }

    pub fn get_entry(&self, index: usize) -> &PatternEntry {
        &self.entries[index]
    }

    pub fn pattern_count(&self) -> usize {
        self.entries.len()
    }
}
