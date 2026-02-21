use crate::models::{
    ResolvedAction, RewritePassThroughAction, RewriteRule, RuleAction, SequenceStepAction,
};
use regex::Regex;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone)]
pub struct NormalizedRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body_text: String,
}

#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub rule: RewriteRule,
    pub specificity: i32,
    pub notes: Vec<String>,
}

pub fn normalize_headers(headers: &http::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|string_value| (key.as_str().to_lowercase(), string_value.to_string()))
        })
        .collect()
}

pub fn parse_query(url: &str) -> HashMap<String, String> {
    Url::parse(url)
        .ok()
        .map(|parsed| {
            parsed
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default()
}

pub fn select_matching_rule(
    rules: &[RewriteRule],
    request: &NormalizedRequest,
) -> Option<MatchCandidate> {
    let mut best: Option<MatchCandidate> = None;

    for rule in rules.iter().filter(|r| r.enabled) {
        let (matched, specificity, notes) = evaluate_rule(rule, request);
        if !matched {
            continue;
        }

        let candidate = MatchCandidate {
            rule: rule.clone(),
            specificity,
            notes,
        };

        best = match best {
            None => Some(candidate),
            Some(current) => {
                if is_better_candidate(&candidate, &current) {
                    Some(candidate)
                } else {
                    Some(current)
                }
            }
        };
    }

    best
}

pub fn resolve_action(
    rule: &RewriteRule,
    sequence_counters: &mut HashMap<String, usize>,
) -> (ResolvedAction, Vec<String>) {
    match &rule.action {
        RuleAction::MockResponse(action) => (
            ResolvedAction::MockResponse(action.clone()),
            vec!["action=mockResponse".to_string()],
        ),
        RuleAction::RewritePassThrough(action) => (
            ResolvedAction::RewritePassThrough(action.clone()),
            vec!["action=rewritePassThrough".to_string()],
        ),
        RuleAction::Sequence(sequence) => {
            if sequence.steps.is_empty() {
                return (
                    ResolvedAction::RewritePassThrough(RewritePassThroughAction {
                        request: None,
                        response: None,
                        delay_ms: None,
                    }),
                    vec!["action=sequence-empty-fallback".to_string()],
                );
            }

            let counter = sequence_counters.entry(rule.id.clone()).or_insert(0);
            let current_index = *counter % sequence.steps.len();
            *counter += 1;

            let notes = vec![format!(
                "action=sequence index={} total={}",
                current_index,
                sequence.steps.len()
            )];

            match &sequence.steps[current_index].action {
                SequenceStepAction::MockResponse(action) => {
                    (ResolvedAction::MockResponse(action.clone()), notes)
                }
                SequenceStepAction::RewritePassThrough(action) => {
                    (ResolvedAction::RewritePassThrough(action.clone()), notes)
                }
            }
        }
    }
}

fn is_better_candidate(next: &MatchCandidate, current: &MatchCandidate) -> bool {
    if next.rule.priority != current.rule.priority {
        return next.rule.priority > current.rule.priority;
    }

    if next.specificity != current.specificity {
        return next.specificity > current.specificity;
    }

    next.rule.id < current.rule.id
}

fn evaluate_rule(rule: &RewriteRule, request: &NormalizedRequest) -> (bool, i32, Vec<String>) {
    let mut specificity = 0;
    let mut notes = vec![];

    if let Some(methods) = &rule.matcher.methods {
        let expected_methods: Vec<String> = methods.iter().map(|m| m.to_uppercase()).collect();
        if !expected_methods.contains(&request.method) {
            return (false, specificity, notes);
        }
        specificity += 20;
        notes.push(format!("method={} matched", request.method));
    }

    if !wildcard_match(&rule.matcher.url_pattern, &request.url) {
        return (false, specificity, notes);
    }

    let wildcard_count = rule
        .matcher
        .url_pattern
        .chars()
        .filter(|c| *c == '*')
        .count() as i32;
    let pattern_chars = rule.matcher.url_pattern.len() as i32;
    let url_specificity_boost = (pattern_chars - (wildcard_count * 3)).max(1);
    specificity += 30 + url_specificity_boost;
    notes.push(format!("urlPattern={} matched", rule.matcher.url_pattern));

    if wildcard_count == 0 {
        specificity += 50;
        notes.push("urlPattern exact".to_string());
    }

    for (key, expected_value) in &rule.matcher.header_equals {
        let normalized_key = key.to_lowercase();
        match request.headers.get(&normalized_key) {
            Some(actual) if actual == expected_value => {
                specificity += 12;
                notes.push(format!("header {} matched", normalized_key));
            }
            _ => return (false, specificity, notes),
        }
    }

    for (key, expected_value) in &rule.matcher.query_equals {
        match request.query.get(key) {
            Some(actual) if actual == expected_value => {
                specificity += 10;
                notes.push(format!("query {} matched", key));
            }
            _ => return (false, specificity, notes),
        }
    }

    if let Some(fragment) = &rule.matcher.body_contains {
        if !request.body_text.contains(fragment) {
            return (false, specificity, notes);
        }
        specificity += 8;
        notes.push("bodyContains matched".to_string());
    }

    (true, specificity, notes)
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let escaped = regex::escape(pattern).replace("\\*", ".*");
    let full_pattern = format!("^{}$", escaped);

    match Regex::new(&full_pattern) {
        Ok(regex) => regex.is_match(text),
        Err(_) => false,
    }
}
