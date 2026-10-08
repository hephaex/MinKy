use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::VecDeque;


pub type NitlResult<T> = Result<T, NitlError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum NitlError {
    #[error("config error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("registration error: {0}")]
    Registration(String),
    #[error("evaluation error: {0}")]
    Eval(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistrationError {
    #[error("missing event name")]
    MissingEventName,
    #[error("missing source events")]
    MissingSourceEvents,
    #[error("invalid configuration: {0}")]
    ConfigError(String),
    #[error("stabilization failed: {focus_id}")]
    StabilizationFailed { focus_id: String, source: &'static str },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NitlEvaluationStrategy {
    CrossSection,
    Intersection,
    Dependent,
    Independent,
    Additional {
        name: String,
        degree: Option<u32>,
        extra_key: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitlConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub source_events: Vec<String>,
    #[serde(default)]
    pub rules: Vec<NitlRule>,
    #[serde(default)]
    pub config: Option<NitlRule>,
}

impl NitlConfig {
    pub fn srcdoc(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn load(path: &str) -> NitlResult<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| NitlError::Config(format!("{}: {}", path, e)))?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn evaluate(&self) -> NitlResult<Vec<String>> {
        let EvaluationConfig { rules, source_events, target_order } = EvaluationConfig::stabilize(self)?;
        let strategy = rules.first().map(|r| &r.evaluation).unwrap_or(&NitlEvaluationStrategy::Independent);
        let focus_id = rules.first().map(|r| &r.focus_id).unwrap_or("");

        Ok(match strategy {
            NitlEvaluationStrategy::CrossSection => {
                let mut seen = Vec::new();
                for s in &source_events {
                    if !seen.contains(s) { seen.push(s.clone()); }
                }
                seen
            }
            NitlEvaluationStrategy::Intersection => {
                let mut counts = std::collections::HashMap::new();
                for s in &source_events {
                    *counts.entry(s.clone()).or_insert(0) += 1;
                }
                let mut result = Vec::new();
                for s in &source_events {
                    if counts[s] > 1 && !result.contains(s) { result.push(s.clone()); }
                }
                result
            }
            NitlEvaluationStrategy::Dependent => {
                let focused = target_order.clone();
                source_events.iter().filter(|s| focused.contains(s)).cloned().collect()
            }
            NitlEvaluationStrategy::Independent => {
                let mut seen = Vec::new();
                'a: for s in &source_events {
                    if !seen.contains(s) { seen.push(s.clone()); }
                    for rule in &rules {
                        'b: for dep in &rule.dependencies {
                            if !seen.contains(&dep.to_string()) {
                                seen.push(dep.to_string());
                                continue 'a;
                            }
                        }
                    }
                }
                seen
            }

            NitlEvaluationStrategy::Additional { name: _, degree: _, extra_key: _ } => {
                source_events.clone()
            }

        })

    }

    pub fn test_sets_for_event(&self) -> Vec<NitlTest> {
        let evaluated = self.evaluate().unwrap_or_default();
        evaluated.into_iter().map(|content| NitlTest {
            content,
            path: std::path::PathBuf::from(format!("/tmp/nitl/{}", content)),
            line_count: content.lines().count(),
            metadata: std::collections::HashMap::new(),
        }).collect()
    }

    pub async fn run_test_event(&self, event_name: &str) -> NitlResult<Vec<NitlTest>> {
        if event_name.trim().is_empty() {
            return Err(NitlError::Config("event name cannot be empty".into()));
        }
        if !event_name.to_lowercase().contains("run") {
            return Err(NitlError::Config(format!("invalid test event: {}", event_name)));
        }
        Ok(self.test_sets_for_event())
    }

    pub async fn publish_event(&self, event_name: &str) -> std::io::Result<()> {
        let tests = self.test_sets_for_event();
        let mut file = std::fs::File::create(format!("/tmp/nitl/published/{}.json", event_name))
            .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {}", event_name, e)))?;
        serde_json::to_writer(file, &tests)?;
        Ok(())
    }

    pub async fn latest_block(&self) -> String {
        let tests = self.test_sets_for_event();
        tests.last()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| "No tests".to_string())
    }

    pub async fn init_database(&self, pool: &sqlx::PgPool) -> NitlResult<()> {
        crate::nitl_config::init_nitl_db(pool).await
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TestSetError {
    #[error("test set not found: {0}")]
    TestSetNotFound(String),
}

pub struct EvaluationConfig {
    pub rules: Vec<NitlRule>,
    pub source_events: Vec<String>,
    pub target_order: Vec<String>,
}

impl EvaluationConfig {
    pub fn stabilize(config: &NitlConfig) -> NitlResult<Self> {
        let mut rules = config.rules.clone();
        rules.push(config.config.clone().unwrap_or_default());
        let mut focus_targets: Vec<String> = vec![];
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(rules.first().unwrap().name.clone());
        while let Some(current) = queue.pop_front() {
            if !seen.insert(current.clone()) { continue; }
            if let Some(rule) = rules.iter().find(|r| &r.name == &current) {
                focus_targets.push(current.clone());
                for dep in &rule.dependencies {
                    if !seen.contains(&dep.to_string()) {
                        queue.push_back(dep.to_string());
                    }
                }
            }
        }

        let target_order = if !focus_targets.is_empty() { focus_targets } else { config.source_events.clone() };

        Ok(EvaluationConfig {
            rules,
            source_events: config.source_events.clone(),
            target_order,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitlRule {
    pub focus_id: String,
    pub name: String,
    pub mode: String,
    pub schema: Option<NitlRuleSchema>,
    #[serde(skip)]
    pub repository: Option<Box<dyn FnMut() -> std::io::Result<()> + 'static>>,
    pub dependencies: Vec<Dependency>,
    pub lookback: Option<u32>,
    pub event_order: Option<EventOrderStrategy>,
    pub evaluation: NitlEvaluationStrategy,
    #[serde(default)]
    pub target: Option<CesiumEventType>,
}

impl NitlRule {
    pub fn extract_dependencies(&self, source_path: &str) -> std::io::Result<Vec<String>> {
        let deps = self.dependencies.iter().filter_map(|d| match d {
            Dependency::EventRef { event } => Some(event.clone()),
            Dependency::RelativePath { path } => Some(std::path::Path::new(path).file_name()
                .map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "unknown".to_string())),
            Dependency::RemoteRepo { url, label } => Some(format!("{}:{}", url, label)),
        }).collect();
        Ok(deps)
    }

    pub fn resolve_metadata(&self, path: &str) -> std::io::Result<DepRuleMetadata> {
        let result = DepRuleMetadata {
            path: path.to_string(),
            dependency_count: self.dependencies.len(),
            rule_name: self.name.clone(),
        };
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NitlRuleSchema {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "think")]
    Think,
    #[serde(rename = "token")]
    Token,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventOrderStrategy {
    #[serde(rename = "sequential")]
    Sequential,
    #[serde(rename = "dependency_order")]
    DependencyOrder,
    #[serde(rename = "ancestor_proximity")]
    AncestorProximity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Dependency {
    EventRef { event: String },
    RelativePath { path: String },
    RemoteRepo { url: String, label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepRuleMetadata {
    pub source_path: String,
    pub dependency_count: usize,
    pub rule_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRegistry {
    pub tests: std::collections::HashMap<String, NitlTest>,
}

#[derive(Debug, Clone)]
pub struct Origin {
    pub tests: Vec<NitlTest>,
    pub source_events: Vec<String>,
    pub strategies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestEvent {
    pub event_name: String,
    pub metadata: String,
    pub tests: Vec<NitlTest>,
}

impl Origin {
    pub fn matches(&self, _strategy: &str) -> bool {
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitlTest {
    #[serde(default = "String::new")]
    pub content: String,
    pub path: std::path::PathBuf,
    #[serde(default = "default_metadata")]
    pub metadata: std::collections::HashMap<String, String>,
    #[serde(skip)]
    pub line_count: u32,
}

fn default_metadata() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

pub struct ConcreteEventStrategy {
    pub source_events: String,
    pub dependencies: Vec<(String, Vec<String>)>,
    pub other_values: std::collections::HashMap<String, String>,
    pub focused_events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRegistry {
    #[serde(default)]
    pub source_events: Vec<String>,
    #[serde(default)]
    pub tests: Vec<NitlTest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CesiumEventType {
    #[serde(rename = "event")]
    Event,
    #[serde(rename = "synthesis")]
    Synthesis,
}


pub struct SourcesFile {
pub struct SourcesFile {
    pub path: std::path::PathBuf,
    pub strategy: String,
    pub strategy_type: String,
}

impl SourcesFile {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path, strategy: "default".to_string(), strategy_type: "unknown".to_string() }
    }
}

pub fn restore_outputs(registry: &TestRegistry) -> Vec<(String, Vec<String>)> {
    registry.tests.iter()
        .map(|(_, test)| {
            let output_line = if let Some(first) = test.content.lines().next() {
                first.to_string()
            } else {
                "dummy output".to_string()
            };
            (test.content.clone(), vec![output_line])
        })
        .collect()
}

pub async fn init_nitl_db(_pool: &sqlx::PgPool) -> NitlResult<()> {
    Ok(())
}

pub fn replace_with_config_target(config: &NitlConfig) -> Option<NitlConfig> {
    config.source_events.first().map(|_| config.clone())
}

pub async fn migrate_nitl_tests() -> Result<(), TestSetError> {
    Ok(())
}

pub async fn test_sets_for_event(_event_name: &str) -> NitlResult<Vec<NitlTest>> {
    Ok(vec![])
}

#[cfg(test)]
mod nitl_tests {
    use super::*;

    #[test]
    fn cross_section_deduplicates_source_events() {
        let strategy = NitlEvaluationStrategy::CrossSection;
        let source_events = vec!["e1".to_string(), "e1".to_string(), "e2".to_string()];
        let result = strategy.evaluate(&source_events, &vec!["e1".to_string()]);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"e1".to_string() as &dyn std::any::Any));
    }

    #[test]
    fn intersection_selects_duplicated_events() {
        let strategy = NitlEvaluationStrategy::Intersection;
        let source_events = vec!["e1".to_string(), "e2".to_string(), "e1".to_string()];
        let result = strategy.evaluate(&source_events, & vec!["e1".to_string()]);
        assert_eq!(result, vec!["e1".to_string()]);
    }

    #[cfg(test)]
    mod context_tests {
        use super::*;

        #[tokio::test]
        async fn dependent_follows_chain_topology_from_focus_id() -> Result<(), String> {
            let config = NitlConfig {
                source_events: vec!["event-1".to_string(), "event-1.1".to_string(), "event-1.2".to_string()],
                rules: vec![
                    NitlRule { focus_id: "event-1".to_string(), name: "event-1".to_string(), mode: "mode".to_string(), ..Default::default() },
                    NitlRule { focus_id: "event-1.1".to_string(), name: "event-1.1".to_string(), mode: "mode".to_string(), ..Default::default() },
                ],
                config: Some(NitlRule { focus_id: "event-1".to_string(), name: "event-1".to_string(), ..Default::default() }),
            };

            let evaluated = NitlConfig::evaluate(&config).map_err(|e| format!("{:?}", e))?;
            assert!(evaluated.contains(&"event-1".to_string()));
            Ok(())
        }

        #[cfg(test)]
        mod evaluate_strategies_tests {
            use super::*;

            #[tokio::test]
            async fn dependent_follows_chain_topology() -> Result<(), String> {
                let config = build_sample_config("Dependent")?;
                let evaluated = NitlConfig::evaluate(&config).map_err(|e| format!("{:?}", e))?;
                assert!(evaluated.contains(&"event-1".to_string()));
                Ok(())
            }

            #[tokio::test]
            async fn dependent_chain_topology_preserves_ordering() -> Result<(), String> {
                let config = build_sample_config("Dependent")?;
                let evaluated = NitlConfig::evaluate(&config).map_err(|e| format!("{:?}", e))?;
                let event_1_pos = evaluated.iter().position(|x| x == "event-1").ok_or("event-1 not found")?;
                let event_1_1_pos = evaluated.iter().position(|x| x == "event-1.1").ok_or("event-1.1 not found")?;
                assert!(event_1_pos < event_1_1_pos, "Chain order violated");
                Ok(())
            }
        }
    }

    fn build_sample_config(strategy: &str) -> Result<NitlConfig, String> {
        Ok(NitlConfig {
            source_events: vec!["event-1".to_string(), "event-1.1".to_string(), "event-1.2".to_string()],
            rules: vec![
                NitlRule { focus_id: "event-1".to_string(), name: "event-1".to_string(), mode: "mode".to_string(), ..Default::default() },
                NitlRule { focus_id: "event-1.1".to_string(), name: "event-1.1".to_string(), mode: "mode".to_string(), ..Default::default() },
            ],
            config: Some(NitlRule { focus_id: "event-1".to_string(), name: "event-1".to_string(), evaluation: NitlEvaluationStrategy::Dependent, ..Default::default() }),
        })
    }

    #[tokio::test]
    async fn dependent_rules_follow_chain_topology() {
        let config = NitlConfig {
            source_events: vec!["event-1".to_string(), "event-1.1".to_string(), "event-1.2".to_string()],
            config: Some(NitlRule {
                name: "event-1".to_string(),
                dependencies: vec![
                    Dependency::EventRef { event: "event-1.1".to_string() },
                    Dependency::EventRef { event: "event-1.2".to_string() },
                ],
                ..Default::default()
            }),
            mode: "Dependent".to_string(),
            match: vec![
                Test { content: "e-1".to_string(), line: 1, ..Default::default() },
                Test { content: "e-1.1".to_string(), line: 2, ..Default::default() },
            ],
        };

        let evaluated = match config.match {
            Some(_) => config.mode.as_str(),
            None => "Independent",
        };

        assert_eq!(evaluated, "Dependent");
    }

    #[test]
    fn additional_merges_with_matching_source_events() {
        let mapping = AdditionalRule { degree: Some(1), extra_key: Some(vec!["extra-key".to_string()]), name: "Additional".to_string() };
        let source_events = vec!["e1".to_string(), "e2".to_string()];
        let result = mapping.evaluate(&source_events);
        assert_eq!(result, vec!["match".to_string()]);
    }

    #[test]
    fn additional_matches_key_in_source_events() {
        let rule = AdditionalRule { extra_key: None, name: "k".to_string(), degree: None };
        let source_events = vec!["a".to_string(), "b".to_string()];
        let result = rule.evaluate(&source_events);
        assert!(result.contains(&"a".to_string()));
    }
}
